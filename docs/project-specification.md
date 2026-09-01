# DutyGrid 專案規格書

**文件版本：** Beta  
**專案版本：** 0.1.0（Beta）  
**狀態：** Beta 發行候選；依目前程式碼實作整理，未實作事項會明確標示為限制或後續項目。Beta 版可供受控試用，不視為已完成機關級正式上線條件。

## 1. 產品定義

DutyGrid 是離線優先（offline-first）的桌面勤務部署規劃系統，協助道路警衛、活動維安與崗哨型勤務完成下列工作：建立勤務計畫、在地圖或勤務簡圖標註點位、繪製路線、匯入及配置人員、整理裝備與部署表，並輸出 PNG、PDF、Excel 及工作區設定檔。

系統定位為單一受信任 OS 帳號下的本機桌面工具。它沒有自建 HTTP API、雲端帳號、同步服務、多人協作或遠端資料庫。

### 1.1 使用者與範圍

- **主要使用者：** 需要製作勤務配置圖與部署表的規劃人員。
- **支援平台：** Tauri 2 桌面應用程式；macOS 開發與驗證完成，設定同時包含 Windows 與 Linux 原生金鑰儲存後端。
- **資料類型：** 勤務計畫、點位、路線、人員基本資料、無線電代號、聯絡電話、配置、裝備與匯出成果。
- **不包含：** 帳號登入、角色權限、遠端備援、簽章式稽核、派勤通知、即時定位與多人即時編輯。

## 2. 技術架構

```text
React 19 + TypeScript + Vite（Tauri WebView）
                 │ invoke()
                 ▼
Tauri 2 / Rust command 邊界
  ├─ SQLCipher + rusqlite：本機資料、migration、驗證、CRUD
  ├─ keyring：macOS Keychain / Windows Credential Manager / Linux Secret Service
  ├─ tauri-plugin-dialog：原生選檔與儲存對話框
  ├─ calamine：CSV/XLSX 人員資料匯入
  └─ zip + XML：標準化 Excel 部署表輸出
                 │
                 ▼
加密 dutygrid.db、受管理底圖、每日 audit log、使用者選定的匯出檔
```

### 2.1 前端

- **框架與語言：** React 19、TypeScript、Vite 8。
- **地圖：** MapLibre GL；預設連線國土測繪中心 WMTS，失敗時使用 OpenStreetMap 圖磚 fallback。
- **兩種繪圖模式：**
  - 地圖模式使用經緯度座標。
  - 自選底圖模式使用 0–1000 的相對 XY 座標，Canvas/SVG 負責縮放、拖曳、點位與路線繪製。
- **延遲載入：** MapLibre 與自選底圖元件以 React `lazy`／`Suspense` 分割，降低首頁初始載入成本。

### 2.2 原生與資料層

- **桌面框架：** Tauri 2，Rust command 是 WebView 與本機資源間的唯一正式邊界。
- **資料庫：** `rusqlite` 搭配 `bundled-sqlcipher-vendored-openssl`；資料庫檔與 WAL 受 SQLCipher 保護。
- **金鑰儲存：** `keyring` 啟用 `apple-native`、`windows-native`、`linux-native-sync-persistent` 與 `crypto-rust` feature。
- **資料庫 migration：** `0001`–`0018`；升版前建立備份、一般 migration 以 transaction 套用、最後執行 foreign-key check。表重建型 migration 仍以備份作為中斷復原保護。

### 2.3 專案模組

| 模組 | 職責 |
| --- | --- |
| `src/app/App.tsx` | 畫面流程、前端狀態、Tauri command 協調、部署表計算、工作區與輸出控制。 |
| `src/features/map/MapCanvas.tsx` | MapLibre 地圖、點位／路線 marker、底圖 fallback、地圖畫面擷取。 |
| `src/features/map/CustomBasemapCanvas.tsx` | 自選底圖編輯、XY 座標、路線和點位繪製。 |
| `src/features/map/CustomBasemapOutput.tsx` | 自選底圖的地圖輸出視圖。 |
| `src-tauri/src/lib.rs` | Tauri command、原生檔案對話框、底圖／工作區檔案、Excel bytes 與輸出保存。 |
| `src-tauri/src/database.rs` | 金鑰與加密開檔、資料驗證、migration、SQL CRUD、人員匯入、audit log。 |
| `src-tauri/migrations/` | 不可回頭修改的 schema 演進檔案。 |
| `標準化部署表.xlsx` | 打包的 Excel 部署表範本。 |

## 3. 功能規格

### 3.1 勤務計畫與工作區

- 建立、列出及刪除勤務計畫。
- 計畫可選擇地圖模式或自選底圖模式。
- 工作區狀態保存目前頁籤、選取點位／路線、部署選項與地圖輸出設定。
- 可輸出及讀取工作區 JSON；其作用是恢復同一台電腦既有計畫的操作狀態，**不是完整資料庫備份，也不支援跨 OS 帳號或搬移加密 DB**。
- 開發版與正式版使用不同 bundle identifier 與不同應用程式資料目錄。

### 3.2 底圖、點位與路線

- 原生選擇 PNG、JPG、JPEG、WEBP 或 SVG 底圖，最大 20 MB；選取後複製到應用程式受管理的 `custom-basemaps/` 目錄。
- 只允許讀取受管理底圖，避免前端透過路徑任意讀取本機檔案。
- 點位包含編號、名稱、備註、色彩、類型、經緯度與可選 XY 座標。
- 支援一般、空心與號誌點位；拖曳移動、改名與刪除。
- 支援兩類路線：由已存在點位依順序組成的點位路線，以及不連結點位的手繪幾何路線。
- 路線支援六種顏色、實線、虛線、實箭頭與虛箭頭；常用路線可跨計畫保存幾何座標。
- 後端驗證文字長度、顏色、點位類型、有限座標、XY 範圍與路線至少兩個頂點。

### 3.3 人員匯入與配置

- 原生選擇 CSV 或 XLSX，人員檔大小上限 10 MB。
- 支援中文／英文標題列，標題可位於前 10 列；CSV 支援 UTF-8（含 BOM）、Big5 與 quoted CSV。
- 必填欄位為員編、無線電代號、姓名、職稱、單位與電話。
- 同員編或同無線電代號的資料會更新既有人員；兩個代號分別命中不同人員時拒絕該列。
- 匯入逐列處理，合法列保存，拒絕列與原因寫入匯入紀錄。
- 人員可配置到同一計畫的多個點位；後端拒絕跨計畫點位／路線／人員關聯。

### 3.4 部署表、裝備與輸出

- 依路線點位產生部署列，亦可加入其他點位或手動崗哨。
- 可選崗哨別、單位、無線電、協調員電話、人員及裝備；點位裝備會保存至資料庫。
- 內建多組勤務裝備預設組合。
- 匯出：
  - 地圖 PNG。
  - 橫向 A4 PNG 與單頁 PDF，含標題及路線圖例。
  - 依 `標準化部署表.xlsx` 填入 Excel；範本最多 33 列（A7:I39）。
- 所有輸出均由 Rust 開啟原生儲存對話框；前端只傳送建議檔名、允許副檔名與 bytes，不可指定任意輸出路徑。

## 4. 資料模型

| 資料表 | 說明 |
| --- | --- |
| `schema_migrations` | 已套用 schema 版本。 |
| `duty_plans` | 勤務計畫與模式、底圖資訊。 |
| `duty_points` | 點位資訊及座標。 |
| `duty_routes`、`duty_route_stops` | 路線與點位順序關聯。 |
| `common_routes` | 可重複使用的路線幾何。 |
| `personnel` | 人員基本資料；有效資料不保留重複的原始 JSON。 |
| `import_batches`、`personnel_import_errors` | 人員匯入批次與拒絕資料列。 |
| `personnel_assignments` | 人員與計畫／點位的配置。 |
| `deployment_equipment` | 計畫＋點位的裝備項目 JSON。 |
| `workspace_states` | 工作區 UI 狀態與部分部署／輸出選項。 |

外鍵已啟用；路線停靠點、配置與計畫關係由後端驗證與 SQLite 約束共同維持。

## 5. 安全規格

### 5.1 資料保護

- 資料庫以 SQLCipher 加密，首次啟動產生 256-bit 隨機 key。
- key 僅放在目前 OS 帳號的憑證儲存區；不寫入資料庫、設定、工作區 JSON 或 audit log。
- 應用程式資料目錄設定為僅擁有者可存取；Unix 目錄為 `0700`、資料庫與 audit 檔為 `0600`。
- 舊明文 DB 先建立加密暫存副本並驗證後取代；若 key 遺失，程式拒絕開啟，不會把加密檔當明文覆寫。
- DB 固定由應用程式資料目錄管理；直接複製、改名或搬移 `dutygrid.db` 不受支援。

### 5.2 WebView 與檔案邊界

- CSP 僅允許應用程式本身、Tauri IPC、國土測繪中心與 OpenStreetMap 所需來源。
- 選檔、底圖匯入、人員匯入與工作區讀取在 Rust 內以原生對話框完成。
- 檔案類型、大小、一般檔案型態與受管理路徑均在 Rust 驗證。
- 匯出限於 JSON、PNG、PDF、XLSX，且最大 50 MB。

### 5.3 稽核紀錄

- 本機 `logs/` 依日建立 JSON Lines audit log。
- 欄位：Unix timestamp、操作、資源類型、筆數、成功狀態。
- 已記錄人員資料讀取／匯入／清除、人員配置異動及檔案匯出。
- log 不記錄姓名、電話、檔案路徑、SQL 或原始匯入資料。

### 5.4 安全限制與責任

- 本機 audit log 不具防竄改、簽章或遠端集中保存能力。
- 本系統沒有帳號、RBAC 或多人隔離；同一 OS 帳號下使用者視為同一受信任主體。
- Keychain／Credential Manager 被清除、OS 帳號遺失或 DB 移至其他帳號時，沒有受控 escrow key 就無法復原資料。
- PNG、PDF、Excel 和工作區 JSON 可能包含人員資訊，不受 SQLCipher 保護；機關應另行定義匯出與保存政策。

## 6. 非功能需求與品質

| 面向 | 現況／要求 |
| --- | --- |
| 離線能力 | 計畫、點位、人員、配置與部署資料皆在本機；地圖模式的圖磚需網路。 |
| 效能 | MapLibre 模組延遲載入；路線查詢以 joined query 避免點位 N+1 查詢；marker 以 registry 更新。 |
| 一致性 | migration、路線建立、種子資料與部分寫入採 transaction；資料庫啟動及 migration 後檢查外鍵。 |
| 可用性 | 原生對話框與中文錯誤訊息；地圖線上來源失敗時 fallback。 |
| 測試 | Rust 資料庫測試 8 項；Vitest 前端整合測試 2 項；TypeScript、Clippy 與生產建置可驗證。 |

## 7. 開發、建置與驗證

| 指令 | 用途 |
| --- | --- |
| `pnpm tauri:dev` | 啟動開發版；使用獨立開發資料目錄。 |
| `pnpm build` | TypeScript 檢查與 Vite 生產建置。 |
| `pnpm test` | Vitest 前端測試加上 TypeScript 檢查。 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust 資料庫與邏輯測試。 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Rust 靜態檢查。 |
| `pnpm tauri build` | 建立可發布桌面包。 |
| `pnpm tauri build --target x86_64-apple-darwin` | 在 Apple Silicon 開發機交叉建立 Intel Mac 安裝包。 |

### 7.1 Beta 發布產物

- **Apple Silicon：** `勤務人力規劃系統_0.1.0_aarch64.dmg`。
- **Intel Mac：** `勤務人力規劃系統_0.1.0_x64.dmg`。
- 兩種架構均已可建置，並以 ad-hoc signing 驗證 bundle 完整性；目前未設定 Apple Developer ID 簽章與 notarization。每次 Beta 重建後，Keychain 可能需要重新確認該包的存取權。對外正式散布前必須補齊 Developer ID 簽章與公證。

## 8. 已知限制與後續建議

1. **原生 E2E：** 現有前端測試為 jsdom 整合測試；尚未以 `tauri-driver` 自動操作真實視窗、系統檔案對話框與 MapLibre 畫面。
2. **稽核完整性：** 若需機關級稽核，應加入 hash chain／簽章、集中收集、保存期限與告警。
3. **身分與權限：** 若同裝置多人共用或需分工，必須加入帳號、RBAC 與受控資料交接。
4. **金鑰復原：** 現況刻意不提供遺失 key 的繞過方式；機關部署應制定 escrow、設備汰換與 OS 帳號重設流程。
5. **匯出資料治理：** 應制定匯出檔的分級、加密、保存期限與銷毀規則。
6. **地圖體積：** MapLibre 動態 chunk 仍大於 Vite 500 KB 警告門檻；不影響功能，但可持續優化圖資與依賴分割。

## 9. 驗收基準

Beta 交付可依以下項目驗收：

- 能建立地圖／自選底圖勤務、點位、路線、人員與部署表。
- 能從 CSV/XLSX 匯入人員並回報拒絕列。
- 能輸出 PNG、PDF、Excel 與工作區 JSON，且輸出位置必須由原生對話框確認。
- 新 DB 與從明文升級的 DB 均以 SQLCipher 開啟，沒有 key 時不可讀取。
- 人員相關操作與輸出會產生不含個資內容的 audit metadata。
- `pnpm test`、`pnpm build`、`cargo test`、`cargo clippy` 均通過。
