# DutyGrid

DutyGrid 是一套以地圖為核心、Desktop-first 的勤務部署規劃工具。

完整的功能、架構、安全與驗收規格請參閱 [專案規格書](docs/project-specification.md)。

目前專案基準資料：

- `標準化部署表.xlsx`：部署表格式參考。
- `data/seeds/personnel-sample.csv`：56 筆虛構人員匯入測試資料。

開發中的應用程式將採用 Tauri、React、TypeScript 與 SQLite。

## 啟動方式

- 已安裝版本：由 macOS「應用程式」開啟「勤務人力規劃系統」，使用正式資料目錄。
- 開發版：執行 `pnpm tauri:dev`，視窗標題會標示「開發版」，並使用獨立的 `tw.gov.newtaipei.dutygrid.development` App 資料目錄與資料庫。
