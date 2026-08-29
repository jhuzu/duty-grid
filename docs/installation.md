# 安裝、啟動與開發

## 已安裝應用程式

README 只明確記載 macOS 的已安裝版本：從「應用程式」開啟「勤務人力規劃系統」。安裝包格式、簽章/公證狀態與最低 macOS 版本未在專案設定中定義，均為**待確認**。

## 原始碼開發環境

已檢查到的必要工具如下：

- Node.js 與 pnpm；`package.json` 偏好 pnpm `^11.21.0`，但設定為 `onFail: ignore`，未強制版本。
- Rust stable toolchain；`src-tauri/Cargo.toml` 使用 Rust edition 2021。
- macOS 與可建置 Tauri 2 的原生依賴。此專案的 Tauri 視窗和 README 均以 macOS 為目標；其他作業系統支援狀態為**待確認**。

在專案根目錄執行：

```sh
pnpm install
pnpm tauri:dev
```

`pnpm tauri:dev` 先啟動 Vite（固定使用 `http://localhost:1420`），再以 `src-tauri/tauri.dev.conf.json` 開啟開發版視窗。開發版產品名稱為「勤務人力規劃系統（開發版）」、identifier 為 `tw.gov.newtaipei.dutygrid.development`，因此與正式版使用不同應用程式資料目錄與 SQLite 資料庫。

僅啟動前端可使用 `pnpm dev`；它不會提供 Tauri 命令，畫面中的本機資料操作會失敗。前端型別/建置檢查使用 `pnpm test`，正式前端建置使用 `pnpm build`。

## 打包

Tauri 設定在建置前執行 `pnpm build`，前端產物位於 `dist/`。可從根目錄執行：

```sh
pnpm tauri build
```

設定會將 `data/reference/banqiao_roads.db` 與 `標準化部署表.xlsx` 打包為資源。不過目前 Rust 程式只在匯出 Excel 時實際讀取 Excel 範本；未找到讀取 `banqiao_roads.db` 的程式碼，因此該資料庫目前對執行功能的用途為**待確認**。

## 關閉

開發模式可在啟動的終端按 Ctrl+C，並關閉 Tauri 視窗。已安裝版本請直接結束應用程式。沒有常駐服務、背景程序或資料庫伺服器。

## 設定與環境變數

未發現 `.env`、`.env.example`、`DATABASE_URL` 或自訂環境變數讀取。`gitignore` 會忽略 `.env` 與 `.env.*`，但沒有範例檔；目前執行不需設定環境變數。

重要設定檔：

- `package.json`：Node 指令與前端相依套件。
- `vite.config.ts`：React 外掛、連接埠 1420、嚴格連接埠與 Rust 目錄監看排除。
- `src-tauri/tauri.conf.json`：產品名稱、bundle identifier、視窗尺寸、建置命令、打包資源與 CSP（目前為 `null`）。
- `src-tauri/tauri.dev.conf.json`：開發版名稱、identifier 與視窗標題覆寫。
- `src-tauri/capabilities/default.json`：預設視窗具備 core 與 dialog 權限。

## 驗證指令

```sh
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
pnpm build
```

第一個指令只是 TypeScript `--noEmit` 檢查；Rust 指令會執行資料庫遷移、裝備保存與 CSV 匯入的三項單元測試。沒有發現前端單元測試、整合測試或端對端測試設定。
