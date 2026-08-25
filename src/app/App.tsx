const navigation = ["勤務計畫", "點位", "路線", "人力配置", "路線調整", "部署表", "人員資料"];

export default function App() {
  return (
    <main className="app-shell">
      <header className="top-bar">
        <strong>DutyGrid</strong>
        <span className="plan-name">尚未開啟勤務計畫</span>
        <div className="top-actions">
          <button type="button" disabled>儲存</button>
          <button type="button" disabled>匯出</button>
        </div>
      </header>
      <aside className="sidebar" aria-label="主要導覽">
        {navigation.map((item, index) => <button className={index === 0 ? "nav-item active" : "nav-item"} key={item} type="button">{item}</button>)}
      </aside>
      <section className="workspace" aria-label="地圖工作區">
        <MapCanvas />
      </section>
      <aside className="inspector" aria-label="詳細資料面板">
        <h1>開始建立勤務計畫</h1>
        <p>建立勤務計畫後，可在地圖上新增勤務點位並安排勤務路線。</p>
        <button type="button">新增勤務計畫</button>
      </aside>
      <footer className="status-bar">資料庫：尚未初始化　｜　路口參考：已隨 App 提供</footer>
    </main>
  );
}
import { MapCanvas } from "../features/map/MapCanvas";
