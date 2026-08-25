# DutyGrid

DutyGrid 是一套以地圖為核心、Desktop-first 的勤務部署規劃工具。

目前專案基準資料：

- `標準化部署表.xlsx`：部署表格式參考。
- `data/reference/banqiao_roads.db`：板橋路口與道路名稱唯讀參考資料。
- `data/seeds/personnel-sample.csv`：56 筆虛構人員匯入測試資料。

開發中的應用程式將採用 Tauri、React、TypeScript 與 SQLite，並以本機 routing engine 處理勤務路線。
