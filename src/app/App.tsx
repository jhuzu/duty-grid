import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { MapCanvas, type MapDutyPoint } from "../features/map/MapCanvas";

const navigation = ["勤務計畫", "點位", "路線", "人力配置", "路線調整", "部署表", "人員資料"];
const pointColors = [
  { value: "red", label: "紅色" }, { value: "orange", label: "橘色" }, { value: "yellow", label: "黃色" },
  { value: "green", label: "綠色" }, { value: "blue", label: "藍色" }, { value: "purple", label: "紫色" },
];
type DutyRoute = { id: string; planId: string; routeName: string; color: string; pointIds: string[]; routeType: string; geometry?: [number, number][]; lineStyle?: "solid" | "dashed" };

export default function App() {
  const [planId, setPlanId] = useState<string | null>(null);
  const [planName, setPlanName] = useState("尚未開啟勤務計畫");
  const [newPlanName, setNewPlanName] = useState("板橋勤務計畫");
  const [message, setMessage] = useState("建立勤務計畫後，可在地圖上新增勤務點位並安排勤務路線。");
  const [points, setPoints] = useState<MapDutyPoint[]>([]);
  const [pendingCoordinate, setPendingCoordinate] = useState<{ latitude: number; longitude: number } | null>(null);
  const [pointCode, setPointCode] = useState("");
  const [pointName, setPointName] = useState("");
  const [pointColor, setPointColor] = useState("red");
  const [activeNav, setActiveNav] = useState("勤務計畫");
  const [pendingDelete, setPendingDelete] = useState<MapDutyPoint | null>(null);
  const [selectedPointId, setSelectedPointId] = useState<string | null>(null);
  const [routes, setRoutes] = useState<DutyRoute[]>([]);
  const [routeName, setRouteName] = useState("");
  const [routeColor, setRouteColor] = useState("blue");
  const [routeLineStyle, setRouteLineStyle] = useState<"solid" | "dashed" | "dotted">("solid");
  const [validationError, setValidationError] = useState("");
  const [routePointIds, setRoutePointIds] = useState<string[]>([]);
  const [isDrawingRoute, setIsDrawingRoute] = useState(false);
  const [manualVertices, setManualVertices] = useState<[number, number][]>([]);
  useEffect(() => { void invoke<Array<{ id: string; planName: string }>>("list_duty_plans").then((plans) => { const latest = plans[0]; if (latest) { setPlanId(latest.id); setPlanName(latest.planName); setMessage("已載入最近勤務計畫。點擊地圖可新增勤務點位。"); } }).catch((error) => setMessage(`無法載入勤務計畫：${String(error)}`)); }, []);
  useEffect(() => { if (planId) void invoke<MapDutyPoint[]>("list_duty_points", { planId }).then(setPoints).catch((error) => setMessage(`無法載入勤務點位：${String(error)}`)); }, [planId]);
  useEffect(() => { if (planId) void invoke<DutyRoute[]>("list_duty_routes", { planId }).then(setRoutes).catch((error) => setMessage(`無法載入勤務路線：${String(error)}`)); }, [planId]);
  useEffect(() => {
    function handleDeleteKey(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null;
      if (target?.matches("input, textarea, [contenteditable='true']") || (event.key !== "Delete" && event.key !== "Backspace")) return;
      if (isDrawingRoute && event.key === " ") { event.preventDefault(); setManualVertices((current) => current.slice(0, -1)); return; }
      if (isDrawingRoute && (event.key === "Delete" || event.key === "Backspace")) { event.preventDefault(); setManualVertices((current) => current.slice(0, -1)); return; }
      if (isDrawingRoute && event.key === "Escape") { event.preventDefault(); setManualVertices([]); setIsDrawingRoute(false); setMessage("已取消手繪路線。"); return; }
      if (isDrawingRoute && event.key === "Enter") { event.preventDefault(); setIsDrawingRoute(false); setMessage("已完成繪製；請命名後保存路線。"); return; }
      if (pendingCoordinate) { event.preventDefault(); setPendingCoordinate(null); setMessage("已取消放置暫存點位。"); return; }
      const point = points.find((item) => item.id === selectedPointId);
      if (point) { event.preventDefault(); setPendingDelete(point); }
    }
    window.addEventListener("keydown", handleDeleteKey);
    return () => window.removeEventListener("keydown", handleDeleteKey);
  }, [isDrawingRoute, pendingCoordinate, points, selectedPointId]);
  async function createPlan() { if (!newPlanName.trim()) { setValidationError("請輸入勤務計畫名稱。"); return; } try { const plan = await invoke<{ id: string; planName: string }>("create_duty_plan", { input: { planName: newPlanName } }); setValidationError(""); setPlanId(plan.id); setPlanName(plan.planName); setMessage("已建立計畫。點擊地圖可新增勤務點位。"); } catch (error) { setMessage(`無法建立勤務計畫：${String(error)}`); } }
  function exitPlan() { setPlanId(null); setPlanName("尚未開啟勤務計畫"); setNewPlanName(""); setPoints([]); setPendingCoordinate(null); setActiveNav("勤務計畫"); setMessage("請輸入勤務計畫名稱以建立新計畫。"); }
  function selectPointLocation(latitude: number, longitude: number) { if (!planId) { setMessage("請先建立勤務計畫，再新增點位。"); return; } if (isDrawingRoute) { setManualVertices((current) => [...current, [longitude, latitude]]); setMessage("已加入路線節點；按 Enter 完成，Backspace 復原。 "); return; } if (activeNav !== "點位") { setMessage("請先從左側選擇「點位」，再於地圖新增勤務點位。"); return; } if (selectedPointId) { setSelectedPointId(null); setMessage("已取消選取勤務點位。"); return; } setPendingCoordinate({ latitude, longitude }); setMessage("已選取地圖位置；請填寫點位編號與名稱後保存。"); }
  function addManualVertex(latitude: number, longitude: number) { setManualVertices((current) => [...current, [longitude, latitude]]); setMessage("已加入路線節點；按 Enter 完成，Backspace 復原。"); }
  async function createPoint() { if (!planId || !pendingCoordinate) return; if (!pointCode.trim() || !pointName.trim()) { setValidationError("請輸入點位編號與崗哨位置。"); return; } try { const point = await invoke<MapDutyPoint>("create_duty_point", { input: { planId, pointCode, pointName, color: pointColor, ...pendingCoordinate } }); setValidationError(""); setPoints((current) => [...current, point]); setPendingCoordinate(null); setPointCode(/^\d+$/.test(point.pointCode) ? String(Number(point.pointCode) + 1) : ""); setPointName(""); setMessage(`已保存點位 ${point.pointCode}。`); } catch (error) { setMessage(String(error)); } }
  async function deletePoint() { if (!pendingDelete) return; try { await invoke("delete_duty_point", { pointId: pendingDelete.id }); setPoints((current) => current.filter((item) => item.id !== pendingDelete.id)); setSelectedPointId(null); setMessage(`已刪除點位 ${pendingDelete.pointCode}。`); setPendingDelete(null); } catch (error) { setMessage(String(error)); } }
  async function movePoint(point: MapDutyPoint, latitude: number, longitude: number) { try { await invoke("move_duty_point", { pointId: point.id, latitude, longitude }); setPoints((current) => current.map((item) => item.id === point.id ? { ...item, latitude, longitude } : item)); setMessage(`已移動點位 ${point.pointCode}；相關路線將在建立後標記為需重算。`); } catch (error) { setMessage(String(error)); } }
  function toggleRoutePoint(pointId: string) { setRoutePointIds((current) => current.includes(pointId) ? current.filter((id) => id !== pointId) : [...current, pointId]); }
  async function createRoute() { if (!planId) return; if (!routeName.trim() || routePointIds.length < 2) { setValidationError("請輸入路線名稱，並至少選擇兩個點位。"); return; } try { const route = await invoke<DutyRoute>("create_duty_route", { input: { planId, routeName, color: routeColor, pointIds: routePointIds } }); setValidationError(""); setRoutes((current) => [...current, { ...route, lineStyle: routeLineStyle }]); setRouteName(""); setRoutePointIds([]); setMessage(`已保存路線 ${route.routeName}。`); } catch (error) { setMessage(String(error)); } }
  async function deleteRoute(route: DutyRoute) { try { await invoke("delete_duty_route", { routeId: route.id }); setRoutes((current) => current.filter((item) => item.id !== route.id)); setMessage(`已刪除路線 ${route.routeName}。`); } catch (error) { setMessage(String(error)); } }
  async function saveManualRoute() { if (!planId) return; if (!routeName.trim() || manualVertices.length < 2) { setValidationError("請輸入路線名稱，並至少繪製兩個折點。"); return; } try { const route = await invoke<DutyRoute>("create_manual_route", { input: { planId, routeName, color: routeColor, geometry: manualVertices } }); setValidationError(""); setRoutes((current) => [...current, { ...route, lineStyle: routeLineStyle }]); setManualVertices([]); setRouteName(""); setMessage(`已保存手繪路線 ${route.routeName}。`); } catch (error) { setMessage(String(error)); } }
  return (
    <main className="app-shell">
      <header className="top-bar">
        <strong>DutyGrid</strong>
        <span className="plan-name">{planName}</span>
        <div className="top-actions">
          <button type="button" disabled>儲存</button>
          <button type="button" disabled>匯出</button>
        </div>
      </header>
      <aside className="sidebar" aria-label="主要導覽">
        {navigation.map((item) => <button className={activeNav === item ? "nav-item active" : "nav-item"} key={item} type="button" onClick={() => { setActiveNav(item); if (item !== "點位") setPendingCoordinate(null); }}>{item}</button>)}
      </aside>
      <section className="workspace" aria-label="地圖工作區">
        <MapCanvas isDrawingRoute={isDrawingRoute} manualVertexColor={routeColor} manualVertices={manualVertices} onMapClick={selectPointLocation} onPendingCancel={() => { setPendingCoordinate(null); setMessage("已取消放置暫存點位。"); }} onPointMoved={movePoint} onPointSelect={setSelectedPointId} onRouteVertex={addManualVertex} pendingColor={pointColor} pendingCoordinate={pendingCoordinate} points={points} routeLines={[...routes.map((route) => ({ color: route.color, dashed: route.lineStyle === "dashed", coordinates: route.geometry ?? route.pointIds.map((id) => points.find((point) => point.id === id)).filter((point): point is MapDutyPoint => Boolean(point)).map((point) => [point.longitude, point.latitude] as [number, number]) })), ...(manualVertices.length > 1 ? [{ color: routeColor, coordinates: manualVertices, dashed: routeLineStyle !== "solid", opacity: 0.45 }] : [])].filter((route) => route.coordinates.length > 1)} selectedPointId={selectedPointId} />
        <div className="map-label-actions"><button type="button" onClick={(event) => { const showing = event.currentTarget.dataset.visible === "true"; document.querySelectorAll(".duty-point-dot").forEach((element) => element.classList.toggle("show-label", !showing)); event.currentTarget.dataset.visible = String(!showing); event.currentTarget.textContent = showing ? "顯示標籤" : "隱藏標籤"; }}>顯示標籤</button></div>
        {activeNav === "路線" && <div className="map-route-colors" aria-label="路線顏色">{pointColors.map((color) => <button aria-label={color.label} className={`color-option ${color.value} ${routeColor === color.value ? "selected" : ""}`} key={color.value} type="button" onClick={() => setRouteColor(color.value)} />)}</div>}
        {activeNav === "路線" && <div className="map-line-style"><button className={routeLineStyle === "solid" ? "active" : ""} type="button" onClick={() => setRouteLineStyle("solid")}>實線</button><button className={routeLineStyle === "dashed" ? "active" : ""} type="button" onClick={() => setRouteLineStyle("dashed")}>虛線</button><button type="button" onClick={() => { setIsDrawingRoute((value) => !value); setMessage(isDrawingRoute ? "已完成繪製；請保存路線。" : "繪製模式：點擊地圖加入折點；空白鍵復原。 "); }}>{isDrawingRoute ? "完成繪製" : "開始繪製"}</button></div>}
      </section>
      <aside className={activeNav === "路線" ? "inspector route-inspector" : "inspector"} aria-label="詳細資料面板" onClick={(event) => { if (event.target === event.currentTarget) setSelectedPointId(null); }}>
        <h1>{activeNav === "點位" ? "勤務點位" : activeNav === "路線" ? "勤務路線" : planId ? "勤務計畫" : "開始建立勤務計畫"}</h1>
        <p>{message}</p>
        {validationError && <p className="validation-error" role="alert">{validationError}</p>}
        {!planId && activeNav === "勤務計畫" && <form onSubmit={(event) => { event.preventDefault(); void createPlan(); }}><label className="field-label" htmlFor="plan-name">勤務計畫名稱</label><input id="plan-name" value={newPlanName} onChange={(event) => { setNewPlanName(event.target.value); setPlanName(event.target.value.trim() || "尚未開啟勤務計畫"); }} /><button type="submit">新增勤務計畫</button></form>}
        {planId && activeNav === "勤務計畫" && <section className="plan-summary"><span>目前開啟</span><strong>{planName}</strong><p>請由左側選擇「點位」以新增、查看或刪除勤務點位。</p><button className="secondary-button" type="button" onClick={exitPlan}>退出勤務計畫</button></section>}
        {activeNav === "點位" && <><div className="point-list">{points.length ? points.map((point) => <div className="point-list-row" key={point.id}><button className={selectedPointId === point.id ? "point-list-item selected" : "point-list-item"} type="button" onClick={() => setSelectedPointId(point.id)}><i className={`point-color-chip ${point.color}`} />{point.pointCode}｜{point.pointName}</button><button className="delete-button" type="button" onClick={() => setPendingDelete(point)}>刪除</button></div>) : <p>尚無勤務點位。請在地圖點擊位置新增。</p>}</div>{pendingCoordinate && <form onSubmit={(event) => { event.preventDefault(); void createPoint(); }}><h2>新增勤務點位</h2><p>{pendingCoordinate.latitude.toFixed(5)}, {pendingCoordinate.longitude.toFixed(5)}</p><label className="field-label">點位編號</label><input value={pointCode} onChange={(event) => setPointCode(event.target.value)} /><label className="field-label">點位名稱（崗哨位置）</label><input value={pointName} onChange={(event) => setPointName(event.target.value)} /><label className="field-label">點位顏色</label><div className="color-options">{pointColors.map((color) => <button aria-label={color.label} className={`color-option ${color.value} ${pointColor === color.value ? "selected" : ""}`} key={color.value} type="button" onClick={() => setPointColor(color.value)} />)}</div><button type="submit">保存點位</button></form>}</>}
        {activeNav === "路線" && <><p>按「開始繪製」後，依序點擊地圖：兩點即成直線，繼續點擊可新增折點。空白鍵復原最後一點。</p>{routes.length > 0 && <><span className="section-caption">已儲存路線</span><div className="route-list">{routes.map((route) => <div className="route-list-row" key={route.id}><span><i className={`point-color-chip ${route.color}`} />{route.routeName}<small>{route.geometry?.length ?? route.pointIds.length} 個節點</small></span><button className="route-delete-button" aria-label={`刪除 ${route.routeName}`} type="button" onClick={() => void deleteRoute(route)}>×</button></div>)}</div></>}<form onSubmit={(event) => { event.preventDefault(); void saveManualRoute(); }}><label className="field-label">路線名稱</label><div className="route-name-row"><input value={routeName} onChange={(event) => setRouteName(event.target.value)} /><button type="submit">保存</button></div><label className="field-label">路線顏色</label><div className="color-options">{pointColors.map((color) => <button aria-label={color.label} className={`color-option ${color.value} ${routeColor === color.value ? "selected" : ""}`} key={color.value} type="button" onClick={() => setRouteColor(color.value)} />)}</div><div className="drawing-actions"><button type="button" onClick={() => { setIsDrawingRoute((value) => !value); setMessage(isDrawingRoute ? "已完成繪製；請保存路線。" : "繪製模式：點擊地圖加入折點；空白鍵復原。 "); }}>{isDrawingRoute ? "完成繪製" : "開始繪製"}</button></div></form><section className="point-route-placeholder"><strong>依點位建立路線</strong><span>功能尚未實作</span></section></>}
      </aside>
      <footer className="status-bar">資料庫：尚未初始化　｜　路口參考：已隨 App 提供</footer>
      {pendingDelete && <div className="confirm-backdrop" role="dialog" aria-modal="true"><section className="confirm-dialog"><h2>刪除勤務點位？</h2><p>將永久刪除「{pendingDelete.pointCode}｜{pendingDelete.pointName}」。</p><div><button type="button" onClick={() => setPendingDelete(null)}>取消</button><button className="delete-button" type="button" onClick={() => void deletePoint()}>確認刪除</button></div></section></div>}
    </main>
  );
}
