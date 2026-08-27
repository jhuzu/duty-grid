import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useRef, useState } from "react";
import dutyGridIcon from "../assets/dutygrid.svg";
import { MapCanvas, type MapDutyPoint } from "../features/map/MapCanvas";

const navigation = ["勤務計畫", "點位", "路線", "人力配置", "部署表", "地圖輸出"];
const pointColors = [
  { value: "red", label: "紅色" }, { value: "orange", label: "橘色" }, { value: "yellow", label: "黃色" },
  { value: "green", label: "綠色" }, { value: "blue", label: "藍色" }, { value: "purple", label: "紫色" },
];
const deploymentPostTypes = ["第一分區\n指揮官", "第一分區\n副指揮官", "便衣哨", "蒐證哨", "第一分區\n督導官", "辨識\n小組", "制高點", "反覘哨", "步巡哨", "協調點\n(制服哨)", "協調點\n(便衣哨)", "攔截圍捕點", "第二分區\n指揮官", "第二分區\n副指揮官", "第二分區\n督導官"];
const deploymentUnits = ["民防組", "板橋監巡區", "便衣預備隊", "督導組", "大觀監巡區", "後埔監巡區", "沙崙監巡區", "行政組", "保防組"];
const equipmentPresets = [
  { name: "制服基本", items: ["制服", "無線電(空氣導管耳機)", "服務證"] },
  { name: "制服加便衣外套基本", items: ["制服加便衣外套(勿穿T-shirt)", "無線電(空氣導管耳機)", "服務證"] },
  { name: "制服外套交通", items: ["制服加便衣外套", "指揮棒", "無線電(空氣導管耳機)", "微型錄影機", "手電筒", "微型攝影機", "哨子"] },
  { name: "便衣蒐證", items: ["便衣(勿穿T-shirt)", "無線電(空氣導管耳機)", "服務證", "微型錄影機", "蒐證器材（攜帶DV、備用電源、斜背包）", "場撿器材"] },
  { name: "便衣攝影", items: ["便衣(勿穿T-shirt)", "無線電(空氣導管耳機)", "服務證", "微型攝影機"] },
  { name: "便衣高處監控", items: ["便衣(勿穿T-shirt)", "攜槍彈", "白帽", "望遠鏡", "無線電(空氣導管耳機)", "服務證", "微型錄影機", "m-police即時傳輸", "蒐證器材（攜帶DV、備用電源、三腳架、橫桿）"] },
  { name: "便衣高處警笛", items: ["便衣(勿穿T-shirt)", "攜槍彈", "白帽", "望遠鏡", "無線電(空氣導管耳機)", "服務證", "微型錄影機", "警笛"] },
  { name: "便衣蒐證場檢", items: ["便衣(勿穿T-shirt)", "攜槍彈", "無線電(空氣導管耳機)", "服務證", "微型錄影機", "m-police即時傳輸", "蒐證器材（攜帶DV、備用電源、三腳架、橫桿）", "場撿器材"] },
  { name: "便衣高處場檢", items: ["便衣(勿穿T-shirt)", "攜槍彈", "白帽", "望遠鏡", "無線電(空氣導管耳機)", "服務證", "微型錄影機", "m-police即時傳輸", "蒐證器材（攜帶DV、備用電源、三腳架、橫桿）", "場撿器材"] },
  { name: "便衣機動", items: ["便衣", "攜槍彈", "無線電(空氣導管耳機)", "微型錄影機", "服務證"] },
  { name: "制服勤務", items: ["制服", "勤務帽", "攜槍彈", "無線電(空氣導管耳機)", "微型錄影機", "服務證"] },
  { name: "制服外套執行", items: ["制服加便衣外套", "勤務帽", "攜槍彈", "指揮棒", "無線電(空氣導管耳機)", "微型錄影機", "移請執行單", "手電筒", "微型攝影機"] },
  { name: "全便衣", items: ["全便衣", "無線電(空氣導管耳機)", "微型錄影機", "服務證", "「板橋分局」識別便帽"] },
  { name: "制服防護", items: ["制服，著防彈衣", "攜槍彈", "無線電(空氣導管耳機)", "指揮棒", "哨子", "反光背心", "微型攝影機", "手電筒"] },
  { name: "制服輕裝", items: ["制服", "勤務帽", "無線電(空氣導管耳機)", "服務證"] },
  { name: "制服交管", items: ["制服", "勤務帽", "螢光背心(新式透氣雨衣備用)", "攜槍彈", "指揮棒", "哨子", "無線電(空氣導管耳機)", "微型攝影機", "雨具"] },
  { name: "制服圍捕", items: ["制服", "攜槍彈", "警笛", "警銬", "微型攝影機", "無線電(空氣導管耳機)", "束帶", "手提擴音器", "滅火器", "警告牌", "防護網", "指揮棒", "哨子"] },
  { name: "制服外套圍捕", items: ["制服加便衣外套", "攜槍彈", "警笛", "微型攝影機", "無線電(空氣導管耳機)"] },
];
const equipmentItems = [...new Set(equipmentPresets.flatMap((preset) => preset.items))];
const banqiaoPoliceStation: [number, number] = [121.4615, 25.0097];
type RouteLineStyle = "solid" | "dashed" | "arrow" | "dashed_arrow";
type DutyRoute = { id: string; planId: string; routeName: string; color: string; pointIds: string[]; routeType: string; geometry?: [number, number][]; lineStyle?: RouteLineStyle };
type CommonRoute = { id: string; routeName: string; color: string; geometry: [number, number][] };
type Personnel = { id: string; personnelCode: string; radioCode: string; name: string; title: string; unit: string; phone: string; isSample: boolean };
type PersonnelAssignment = { id: string; planId: string; personnelId: string; dutyPointId: string | null; assignedUnit: string; assignedTitle: string };
type PersonnelImportLog = { sourceFileName: string; totalRows: number; acceptedRows: number; rejectedRows: number; errors: Array<{ rowNumber: number; errorReason: string; rawRowJson: string }> };
type DeploymentEquipment = { planId: string; dutyPointId: string; selectedItems: string[] };
type ManualDeploymentRow = { id: string; postType: string; location: string; unit: string; personnelIds: string[]; equipment: string[] };
type DeploymentTableRow = { index: number; point: MapDutyPoint; postType: string; units: string; count: number; names: string; radios: string; equipment: string[]; source: "point" | "manual"; manualRow?: ManualDeploymentRow };
type DeploymentPointOverrides = Record<string, { addedIds: string[]; excludedIds: string[] }>;
type WorkspaceState = { planId: string; activeNav: string; selectedPointId: string | null; selectedRouteId: string | null; deploymentRouteId: string | null; deploymentChoices: Record<string, { postType: string; unit: string }>; mapOutputTitle: string; mapOutputZoom: number; mapOutputBearing: number };
type WorkspaceFile = { version: 1; planId: string; planName: string; workspace: Omit<WorkspaceState, "planId"> };
function isPointNearRoute(point: MapDutyPoint, coordinates: [number, number][]) {
  if (coordinates.length < 2) return false;
  const latitudeScale = 111_320;
  const longitudeScale = latitudeScale * Math.cos(point.latitude * Math.PI / 180);
  const px = point.longitude * longitudeScale; const py = point.latitude * latitudeScale;
  return coordinates.slice(1).some(([longitude, latitude], index) => {
    const [startLongitude, startLatitude] = coordinates[index];
    const ax = startLongitude * longitudeScale; const ay = startLatitude * latitudeScale;
    const bx = longitude * longitudeScale; const by = latitude * latitudeScale;
    const dx = bx - ax; const dy = by - ay;
    const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy || 1)));
    return Math.hypot(px - (ax + t * dx), py - (ay + t * dy)) <= 30;
  });
}

export default function App() {
  const mapExporter = useRef<(() => string | null) | null>(null);
  const mapOutputExporter = useRef<(() => string | null) | null>(null);
  const [planId, setPlanId] = useState<string | null>(null);
  const [planName, setPlanName] = useState("尚未開啟勤務計畫");
  const [dutyPlans, setDutyPlans] = useState<Array<{ id: string; planName: string }>>([]);
  const [workspaceReadyPlanId, setWorkspaceReadyPlanId] = useState<string | null>(null);
  const [newPlanName, setNewPlanName] = useState("板橋勤務計畫");
  const [message, setMessage] = useState("建立勤務計畫後，可在地圖上新增勤務點位並安排勤務路線。");
  const [mapCoordinate, setMapCoordinate] = useState<{ latitude: number; longitude: number } | null>(null);
  const [points, setPoints] = useState<MapDutyPoint[]>([]);
  const [pendingCoordinate, setPendingCoordinate] = useState<{ latitude: number; longitude: number; pointType: "duty" | "hollow" | "signal"; color: string } | null>(null);
  const [pointCode, setPointCode] = useState("");
  const [pointName, setPointName] = useState("");
  const [pointColor, setPointColor] = useState("red");
  const [pointType, setPointType] = useState<"duty" | "hollow" | "signal">("duty");
  const [activeNav, setActiveNav] = useState("勤務計畫");
  const [pendingDelete, setPendingDelete] = useState<MapDutyPoint | null>(null);
  const [selectedPointId, setSelectedPointId] = useState<string | null>(null);
  const [routes, setRoutes] = useState<DutyRoute[]>([]);
  const [commonRoutes, setCommonRoutes] = useState<CommonRoute[]>([]);
  const [personnel, setPersonnel] = useState<Personnel[]>([]);
  const [personnelAssignments, setPersonnelAssignments] = useState<PersonnelAssignment[]>([]);
  const [personnelImportLog, setPersonnelImportLog] = useState<PersonnelImportLog | null>(null);
  const [personnelKeyword, setPersonnelKeyword] = useState("");
  const [personnelUnit, setPersonnelUnit] = useState("");
  const [personnelTitle, setPersonnelTitle] = useState("");
  const [showPointLabels, setShowPointLabels] = useState(false);
  const [showPersonnelLabels, setShowPersonnelLabels] = useState(false);
  const [selectedPersonnelLabelPointId, setSelectedPersonnelLabelPointId] = useState<string | null>(null);
  const [deploymentChoices, setDeploymentChoices] = useState<Record<string, { postType: string; unit: string }>>({});
  const [deploymentEquipment, setDeploymentEquipment] = useState<Record<string, string[]>>({});
  const [manualDeploymentRows, setManualDeploymentRows] = useState<ManualDeploymentRow[]>([]);
  const [mergedManualDeploymentRows, setMergedManualDeploymentRows] = useState<ManualDeploymentRow[]>([]);
  const [deploymentPointOverrides, setDeploymentPointOverrides] = useState<DeploymentPointOverrides>({});
  const [deploymentPointToAddId, setDeploymentPointToAddId] = useState("");
  const [manualPersonnelEditorRowId, setManualPersonnelEditorRowId] = useState<string | null>(null);
  const [manualPersonnelKeyword, setManualPersonnelKeyword] = useState("");
  const [manualPersonnelUnit, setManualPersonnelUnit] = useState("");
  const [manualPersonnelTitle, setManualPersonnelTitle] = useState("");
  const [manualEquipmentEditorRowId, setManualEquipmentEditorRowId] = useState<string | null>(null);
  const [equipmentEditorPointId, setEquipmentEditorPointId] = useState<string | null>(null);
  const [mapOutputTitle, setMapOutputTitle] = useState("");
  const [mapOutputZoom, setMapOutputZoom] = useState(0);
  const [mapOutputBearing, setMapOutputBearing] = useState(90);
  const [selectedRouteId, setSelectedRouteId] = useState<string | null>(null);
  const [deploymentRouteId, setDeploymentRouteId] = useState<string | null>(null);
  const [routeName, setRouteName] = useState("");
  const [routeColor, setRouteColor] = useState("blue");
  const [routeLineStyle, setRouteLineStyle] = useState<RouteLineStyle>("solid");
  const [validationError, setValidationError] = useState("");
  const [routePointIds, setRoutePointIds] = useState<string[]>([]);
  const [isDrawingRoute, setIsDrawingRoute] = useState(false);
  const [isPlacingPoint, setIsPlacingPoint] = useState(false);
  const [pointDrawReminder, setPointDrawReminder] = useState(false);
  const [manualVertices, setManualVertices] = useState<[number, number][]>([]);
  useEffect(() => { void invoke<Array<{ id: string; planName: string }>>("list_duty_plans").then((plans) => { setDutyPlans(plans); const latest = plans[0]; if (latest) { setPlanId(latest.id); setPlanName(latest.planName); setMessage("已載入最近勤務計畫及其工作區狀態。"); } }).catch((error) => setMessage(`無法載入勤務計畫：${String(error)}`)); }, []);
  useEffect(() => { if (planId) void invoke<MapDutyPoint[]>("list_duty_points", { planId }).then(setPoints).catch((error) => setMessage(`無法載入勤務點位：${String(error)}`)); }, [planId]);
  useEffect(() => { if (planId) void invoke<DutyRoute[]>("list_duty_routes", { planId }).then(setRoutes).catch((error) => setMessage(`無法載入勤務路線：${String(error)}`)); setSelectedRouteId(null); setDeploymentRouteId(null); }, [planId]);
  useEffect(() => { const route = routes.find((item) => item.id === selectedRouteId); if (route) setRouteLineStyle(route.lineStyle ?? "solid"); }, [routes, selectedRouteId]);
  useEffect(() => { void invoke<CommonRoute[]>("list_common_routes").then(setCommonRoutes).catch((error) => setMessage(`無法載入常用路線：${String(error)}`)); }, []);
  useEffect(() => { void invoke<Personnel[]>("list_personnel").then(setPersonnel).catch((error) => setMessage(`無法載入人員資料：${String(error)}`)); }, []);
  useEffect(() => { void invoke<PersonnelImportLog | null>("latest_personnel_import_log").then(setPersonnelImportLog).catch(() => {}); }, []);
  useEffect(() => { if (planId) void invoke<PersonnelAssignment[]>("list_personnel_assignments", { planId }).then(setPersonnelAssignments).catch((error) => setMessage(`無法載入人力配置：${String(error)}`)); else setPersonnelAssignments([]); }, [planId]);
  useEffect(() => { if (planId) void invoke<DeploymentEquipment[]>("list_deployment_equipment", { planId }).then((items) => setDeploymentEquipment(Object.fromEntries(items.map((item) => [item.dutyPointId, item.selectedItems])))).catch((error) => setMessage(`無法載入部署裝備：${String(error)}`)); else setDeploymentEquipment({}); }, [planId]);
  useEffect(() => {
    if (activeNav !== "部署表") return;
    setDeploymentRouteId((current) => routes.some((route) => route.id === current) ? current : routes.find((route) => route.pointIds.length > 0)?.id ?? routes[0]?.id ?? null);
  }, [activeNav, routes]);
  useEffect(() => {
    if (!planId) return;
    setWorkspaceReadyPlanId(null);
    void invoke<WorkspaceState | null>("load_workspace_state", { planId }).then((saved) => {
      if (saved) {
        setDeploymentChoices({});
        setMapOutputTitle(saved.mapOutputTitle ?? "");
        setMapOutputZoom(saved.mapOutputZoom ?? 0);
        setMapOutputBearing(saved.mapOutputBearing ?? 90);
        setSelectedPointId(saved.selectedPointId);
        setSelectedRouteId(saved.selectedRouteId);
        setDeploymentRouteId(saved.deploymentRouteId ?? saved.selectedRouteId);
        if (navigation.includes(saved.activeNav)) setActiveNav(saved.activeNav);
      }
      setWorkspaceReadyPlanId(planId);
    }).catch((error) => setMessage(`無法載入勤務工作區狀態：${String(error)}`));
  }, [planId]);
  useEffect(() => {
    if (!planId || workspaceReadyPlanId !== planId) return;
    const timer = window.setTimeout(() => {
      void invoke("save_workspace_state", { input: { planId, activeNav, selectedPointId, selectedRouteId, deploymentRouteId, deploymentChoices, mapOutputTitle, mapOutputZoom, mapOutputBearing } }).catch((error) => setMessage(`無法自動儲存工作區狀態：${String(error)}`));
    }, 800);
    return () => window.clearTimeout(timer);
  }, [activeNav, deploymentChoices, deploymentRouteId, mapOutputBearing, mapOutputTitle, mapOutputZoom, planId, selectedPointId, selectedRouteId, workspaceReadyPlanId]);
  useEffect(() => {
    function handleDeleteKey(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null;
      if (target?.matches("input, textarea, [contenteditable='true']")) return;
      if (isDrawingRoute && event.key === " ") { event.preventDefault(); setManualVertices((current) => current.slice(0, -1)); return; }
      if (isDrawingRoute && (event.key === "Delete" || event.key === "Backspace")) { event.preventDefault(); setManualVertices((current) => current.slice(0, -1)); return; }
      if (isDrawingRoute && event.key === "Escape") { event.preventDefault(); setManualVertices([]); setIsDrawingRoute(false); setMessage("已取消手繪路線。"); return; }
      if (isDrawingRoute && event.key === "Enter") { event.preventDefault(); setIsDrawingRoute(false); setMessage("請輸入路線名稱保存路線。"); return; }
      if (event.key !== "Delete" && event.key !== "Backspace") return;
      if (pendingCoordinate) { event.preventDefault(); setPendingCoordinate(null); setMessage("已取消放置暫存點位。"); return; }
      const point = points.find((item) => item.id === selectedPointId);
      if (point) { event.preventDefault(); setPendingDelete(point); }
    }
    window.addEventListener("keydown", handleDeleteKey);
    return () => window.removeEventListener("keydown", handleDeleteKey);
  }, [isDrawingRoute, pendingCoordinate, points, selectedPointId]);
  async function createPlan() { if (!newPlanName.trim()) { setValidationError("請輸入勤務計畫名稱。"); return; } try { const plan = await invoke<{ id: string; planName: string }>("create_duty_plan", { input: { planName: newPlanName } }); setValidationError(""); setDutyPlans((current) => [plan, ...current]); setPlanId(plan.id); setPlanName(plan.planName); setMessage("已建立計畫。點擊地圖可新增勤務點位。"); } catch (error) { setMessage(`無法建立勤務計畫：${String(error)}`); } }
  function openSavedWorkspace(plan: { id: string; planName: string }) { setPlanId(plan.id); setPlanName(plan.planName); setActiveNav("勤務計畫"); setMessage(`正在開啟「${plan.planName}」的已儲存工作區。`); }
  function exitPlan() { setPlanId(null); setPlanName("尚未開啟勤務計畫"); setNewPlanName(""); setPoints([]); setRoutes([]); setPersonnelAssignments([]); setDeploymentChoices({}); setDeploymentEquipment({}); setPendingCoordinate(null); setSelectedPointId(null); setSelectedRouteId(null); setDeploymentRouteId(null); setSelectedPersonnelLabelPointId(null); setEquipmentEditorPointId(null); setManualVertices([]); setIsDrawingRoute(false); setActiveNav("勤務計畫"); setMessage("請輸入勤務計畫名稱以建立新計畫。"); }
  function selectPointLocation(latitude: number, longitude: number) { if (!planId) { setMessage("請先建立勤務計畫，再新增點位。"); return; } if (activeNav === "路線" && isDrawingRoute) { setManualVertices((current) => [...current, [longitude, latitude]]); setMessage("已加入路線節點；完成後請點選「完成繪製」。"); return; } if (activeNav === "路線") { setMessage(manualVertices.length >= 2 ? "請輸入路線名稱保存路線。" : "尚未點選「開始繪製」。"); return; } if (activeNav === "部署表") { setSelectedPointId(null); setSelectedPersonnelLabelPointId(null); setMessage("已取消部署表點位選取。 "); return; } if (activeNav !== "點位") { setMessage("請先從頂部導覽選擇「點位」，再於地圖新增勤務點位。"); return; } if (!isPlacingPoint) { setPointDrawReminder(true); setMessage("尚未點選「開始繪製」，無法繪製"); return; } if (selectedPointId) { setSelectedPointId(null); setMessage("已取消選取勤務點位。"); return; } const selectedColor = pointType === "signal" ? "yellow" : pointColor; setPendingCoordinate({ latitude, longitude, pointType, color: selectedColor }); setMessage(`已選取地圖位置；將新增${pointType === "signal" ? "號誌" : pointType === "hollow" ? "空心圓" : "一般"}點位，請填寫編號與名稱後保存。`); }
  function addManualVertex(latitude: number, longitude: number) { setManualVertices((current) => [...current, [longitude, latitude]]); setMessage("已加入路線節點；完成後請點選「完成繪製」。"); }
  async function createPoint() { if (!planId || !pendingCoordinate) return; if (!pointCode.trim() || !pointName.trim()) { setValidationError("請輸入點位編號與位置名稱。"); return; } const { pointType: savedPointType, color: savedColor, latitude, longitude } = pendingCoordinate; try { const point = await invoke<MapDutyPoint>("create_duty_point", { input: { planId, pointCode, pointName, color: savedColor, pointType: savedPointType, latitude, longitude } }); setValidationError(""); setPoints((current) => [...current, point]); setPendingCoordinate(null); setIsPlacingPoint(false); setPointCode(/^\d+$/.test(point.pointCode) ? String(Number(point.pointCode) + 1) : ""); setPointName(""); setPointType("duty"); setMessage(`已保存${point.pointType === "signal" ? "號誌" : point.pointType === "hollow" ? "空心圓" : "勤務"}點位 ${point.pointCode}。`); } catch (error) { setMessage(String(error)); } }
  async function deletePoint() { if (!pendingDelete) return; try { await invoke("delete_duty_point", { pointId: pendingDelete.id }); setPoints((current) => current.filter((item) => item.id !== pendingDelete.id)); setSelectedPointId(null); setMessage(`已刪除點位 ${pendingDelete.pointCode}。`); setPendingDelete(null); } catch (error) { setMessage(String(error)); } }
  async function movePoint(point: MapDutyPoint, latitude: number, longitude: number) { try { await invoke("move_duty_point", { pointId: point.id, latitude, longitude }); setPoints((current) => current.map((item) => item.id === point.id ? { ...item, latitude, longitude } : item)); setMessage(`已移動點位 ${point.pointCode}；相關路線將在建立後標記為需重算。`); } catch (error) { setMessage(String(error)); } }
  function toggleRoutePoint(pointId: string) { setRoutePointIds((current) => current.includes(pointId) ? current.filter((id) => id !== pointId) : [...current, pointId]); }
  async function createRoute() { if (!planId) return; if (!routeName.trim() || routePointIds.length < 2) { setValidationError("請輸入路線名稱，並至少選擇兩個點位。"); return; } try { const route = await invoke<DutyRoute>("create_duty_route", { input: { planId, routeName, color: routeColor, pointIds: routePointIds, lineStyle: routeLineStyle } }); setValidationError(""); setRoutes((current) => [...current, route]); setRouteName(""); setRoutePointIds([]); setMessage(`已保存路線 ${route.routeName}。`); } catch (error) { setMessage(String(error)); } }
  async function deleteRoute(route: DutyRoute) { try { await invoke("delete_duty_route", { routeId: route.id }); setRoutes((current) => current.filter((item) => item.id !== route.id)); if (selectedRouteId === route.id) setSelectedRouteId(null); setMessage(`已刪除路線 ${route.routeName}。`); } catch (error) { setMessage(String(error)); } }
  async function updateRouteColor(color: string) { setRouteColor(color); const route = routes.find((item) => item.id === selectedRouteId); if (!route) return; const previousColor = route.color; setRoutes((current) => current.map((item) => item.id === route.id ? { ...item, color } : item)); try { await invoke("update_duty_route_color", { routeId: route.id, color }); setMessage(`已更新路線 ${route.routeName} 的顏色。`); } catch (error) { setRoutes((current) => current.map((item) => item.id === route.id ? { ...item, color: previousColor } : item)); setRouteColor(previousColor); setMessage(`無法更新路線顏色：${String(error)}`); } }
  async function updateRouteLineStyle(lineStyle: RouteLineStyle) { setRouteLineStyle(lineStyle); const route = routes.find((item) => item.id === selectedRouteId); if (!route) return; const previousLineStyle = route.lineStyle ?? "solid"; setRoutes((current) => current.map((item) => item.id === route.id ? { ...item, lineStyle } : item)); try { await invoke("update_duty_route_line_style", { routeId: route.id, lineStyle }); setMessage(`已更新路線 ${route.routeName} 為${lineStyle === "solid" ? "實線" : lineStyle === "dashed" ? "虛線" : lineStyle === "arrow" ? "實箭頭線" : "虛箭頭線"}。`); } catch (error) { setRoutes((current) => current.map((item) => item.id === route.id ? { ...item, lineStyle: previousLineStyle } : item)); setRouteLineStyle(previousLineStyle); setMessage(`無法更新路線樣式：${String(error)}`); } }
  function routeCoordinates(route: DutyRoute) { return route.geometry ?? route.pointIds.map((id) => points.find((point) => point.id === id)).filter((point): point is MapDutyPoint => Boolean(point)).map((point) => [point.longitude, point.latitude] as [number, number]); }
  async function saveSelectedRouteAsCommon() { const route = routes.find((item) => item.id === selectedRouteId); if (!route) { setValidationError("請先選擇一條已儲存路線。"); return; } const geometry = routeCoordinates(route); if (geometry.length < 2) { setValidationError("此路線缺少可用節點，無法儲存為常用路線。"); return; } try { const commonRoute = await invoke<CommonRoute>("create_common_route", { input: { routeName: route.routeName, color: route.color, geometry } }); setCommonRoutes((current) => [commonRoute, ...current]); setValidationError(""); setMessage(`已將 ${route.routeName} 儲存為常用路線。`); } catch (error) { setMessage(`無法儲存常用路線：${String(error)}`); } }
  async function applyCommonRoute(commonRoute: CommonRoute) { if (!planId) { setMessage("請先建立勤務計畫，再套用常用路線。"); return; } try { const route = await invoke<DutyRoute>("create_manual_route", { input: { planId, routeName: commonRoute.routeName, color: commonRoute.color, geometry: commonRoute.geometry } }); setRoutes((current) => [...current, { ...route, lineStyle: "solid" }]); setMessage(`已套用常用路線 ${commonRoute.routeName}。`); } catch (error) { setMessage(`無法套用常用路線：${String(error)}`); } }
  async function deleteCommonRoute(commonRoute: CommonRoute) { try { await invoke("delete_common_route", { routeId: commonRoute.id }); setCommonRoutes((current) => current.filter((item) => item.id !== commonRoute.id)); setMessage(`已刪除常用路線 ${commonRoute.routeName}。`); } catch (error) { setMessage(`無法刪除常用路線：${String(error)}`); } }
  async function assignPersonnel(person: Personnel) { if (!planId || !selectedPointId) { setValidationError("請先在地圖選擇要配置的勤務點位。"); return; } const routeKeys = (pointId: string | null) => { const memberships = routes.filter((route) => pointId && route.pointIds.includes(pointId)).map((route) => route.id); return memberships.length ? memberships : [`point:${pointId ?? "unassigned"}`]; }; const selectedRouteKeys = routeKeys(selectedPointId); const existing = personnelAssignments.find((assignment) => assignment.personnelId === person.id && routeKeys(assignment.dutyPointId).some((key) => selectedRouteKeys.includes(key))); try { if (existing) { await invoke("move_personnel_assignment", { assignmentId: existing.id, dutyPointId: selectedPointId }); setPersonnelAssignments((current) => current.map((assignment) => assignment.id === existing.id ? { ...assignment, dutyPointId: selectedPointId } : assignment)); setMessage(`已將 ${person.name} 移至目前勤務點位。`); } else { const assignment = await invoke<PersonnelAssignment>("create_personnel_assignment", { input: { planId, personnelId: person.id, dutyPointId: selectedPointId, assignedUnit: person.unit, assignedTitle: person.title } }); setPersonnelAssignments((current) => [...current, assignment]); setMessage(`已將 ${person.name} 配置至勤務點位。`); } setValidationError(""); } catch (error) { setMessage(`無法配置人員：${String(error)}`); } }
  async function removePersonnelAssignment(assignment: PersonnelAssignment) { try { await invoke("delete_personnel_assignment", { assignmentId: assignment.id }); setPersonnelAssignments((current) => current.filter((item) => item.id !== assignment.id)); setMessage("已移除人力配置。"); } catch (error) { setMessage(`無法移除人力配置：${String(error)}`); } }
  async function refreshPersonnelImportLog() { setPersonnelImportLog(await invoke<PersonnelImportLog | null>("latest_personnel_import_log")); }
  async function choosePersonnelFile() { try { const path = await open({ multiple: false, filters: [{ name: "人力資料", extensions: ["csv", "xlsx"] }] }); if (typeof path !== "string") { setMessage("已取消選擇人力資料檔。 "); return; } const result = await invoke<{ totalRows: number; acceptedRows: number; rejectedRows: number }>("import_personnel_file", { path }); setPersonnel(await invoke<Personnel[]>("list_personnel")); await refreshPersonnelImportLog(); setMessage(`人力匯入完成：${result.acceptedRows}/${result.totalRows} 筆成功，${result.rejectedRows} 筆拒絕。`); } catch (error) { setMessage(`無法人力匯入：${String(error)}`); } }
  async function importDefaultPersonnelFile() { try { const result = await invoke<{ totalRows: number; acceptedRows: number; rejectedRows: number }>("import_default_personnel_file"); setPersonnel(await invoke<Personnel[]>("list_personnel")); await refreshPersonnelImportLog(); setMessage(`測試人力匯入完成：${result.acceptedRows}/${result.totalRows} 筆成功，${result.rejectedRows} 筆拒絕。`); } catch (error) { setMessage(`無法讀取測試人力資料：${String(error)}`); } }
  const activePersonnel = personnel.filter((person) => !person.isSample);
  const units = [...new Set(activePersonnel.map((person) => person.unit))].sort();
  const titles = [...new Set(activePersonnel.map((person) => person.title))].sort();
  const selectedPoint = points.find((point) => point.id === selectedPointId);
  const filteredPersonnel = activePersonnel.filter((person) => { const keyword = personnelKeyword.trim().toLowerCase(); return (!keyword || [person.name, person.personnelCode, person.radioCode].some((value) => value.toLowerCase().includes(keyword))) && (!personnelUnit || person.unit === personnelUnit) && (!personnelTitle || person.title === personnelTitle); });
  const pointAssignments = personnelAssignments.filter((assignment) => assignment.dutyPointId === selectedPointId && activePersonnel.some((person) => person.id === assignment.personnelId));
  const personnelLabels = personnelAssignments.reduce<Record<string, string[]>>((labels, assignment) => { if (!assignment.dutyPointId) return labels; const person = activePersonnel.find((item) => item.id === assignment.personnelId); if (person) (labels[assignment.dutyPointId] ??= []).push(person.name); return labels; }, {});
  const sameCodeAssignments = selectedPoint ? personnelAssignments.filter((assignment) => { const point = points.find((item) => item.id === assignment.dutyPointId); return point && point.id !== selectedPoint.id && point.pointCode === selectedPoint.pointCode && point.color !== selectedPoint.color; }) : [];
  const preferredPersonnelIds = new Set(sameCodeAssignments.map((assignment) => assignment.personnelId));
  const prioritizedPersonnel = [...filteredPersonnel].sort((left, right) => Number(preferredPersonnelIds.has(right.id)) - Number(preferredPersonnelIds.has(left.id)) || left.personnelCode.localeCompare(right.personnelCode));
  async function saveManualRoute() { if (!planId) return; if (!routeName.trim()) { setValidationError("請輸入路線名稱。"); return; } if (manualVertices.length < 2) { setValidationError("請至少繪製兩個折點後再保存路線。"); return; } try { const route = await invoke<DutyRoute>("create_manual_route", { input: { planId, routeName, color: routeColor, geometry: manualVertices } }); if (routeLineStyle !== "solid") await invoke("update_duty_route_line_style", { routeId: route.id, lineStyle: routeLineStyle }); setValidationError(""); setRoutes((current) => [...current, { ...route, lineStyle: routeLineStyle }]); setManualVertices([]); setRouteName(""); setMessage(`已保存手繪路線 ${route.routeName}。`); } catch (error) { setMessage(String(error)); } }
  const deploymentRoute = routes.find((route) => route.id === deploymentRouteId) ?? null;
  const baseDeploymentPoints = deploymentRoute ? deploymentRoute.pointIds.length ? deploymentRoute.pointIds.map((id) => points.find((point) => point.id === id)).filter((point): point is MapDutyPoint => Boolean(point)) : points.filter((point) => isPointNearRoute(point, routeCoordinates(deploymentRoute))) : [];
  const deploymentPointOverride = deploymentRoute ? deploymentPointOverrides[deploymentRoute.id] ?? { addedIds: [], excludedIds: [] } : { addedIds: [], excludedIds: [] };
  const deploymentPoints = [...baseDeploymentPoints, ...deploymentPointOverride.addedIds.map((id) => points.find((point) => point.id === id)).filter((point): point is MapDutyPoint => Boolean(point))].filter((point, index, rows) => !deploymentPointOverride.excludedIds.includes(point.id) && rows.findIndex((item) => item.id === point.id) === index);
  const mapPoints = points;
  const mapRouteLines = activeNav === "部署表" && deploymentRoute
    ? [{ color: deploymentRoute.color, dashed: deploymentRoute.lineStyle === "dashed" || deploymentRoute.lineStyle === "dashed_arrow", arrow: deploymentRoute.lineStyle === "arrow" || deploymentRoute.lineStyle === "dashed_arrow", coordinates: routeCoordinates(deploymentRoute) }].filter((route) => route.coordinates.length > 1)
    : [...routes.map((route) => ({ color: route.color, dashed: route.lineStyle === "dashed" || route.lineStyle === "dashed_arrow", arrow: route.lineStyle === "arrow" || route.lineStyle === "dashed_arrow", coordinates: routeCoordinates(route) })), ...(manualVertices.length > 1 ? [{ color: routeColor, coordinates: manualVertices, dashed: routeLineStyle === "dashed" || routeLineStyle === "dashed_arrow", arrow: routeLineStyle === "arrow" || routeLineStyle === "dashed_arrow", opacity: 0.45 }] : [])].filter((route) => route.coordinates.length > 1);
  const pointDeploymentRows: DeploymentTableRow[] = deploymentPoints.map((point, index) => {
    const assignments = personnelAssignments.filter((assignment) => assignment.dutyPointId === point.id && activePersonnel.some((person) => person.id === assignment.personnelId));
    const assignedPeople = assignments.map((assignment) => activePersonnel.find((person) => person.id === assignment.personnelId)).filter((person): person is Personnel => Boolean(person));
    const choice = deploymentChoices[point.id];
    const equipment = deploymentEquipment[point.id] ?? [];
    return { index: index + 1, point, postType: choice?.postType ?? "", units: choice?.unit ?? [...new Set(assignments.map((assignment) => assignment.assignedUnit))].join("／"), count: assignedPeople.length, names: assignedPeople.map((person) => `${person.title}\n${person.name}\n${person.phone}`).join("\n\n") || "—", radios: assignedPeople.map((person) => person.radioCode).join("\n") || "—", equipment, source: "point" };
  });
  const deploymentRows: DeploymentTableRow[] = [...pointDeploymentRows, ...mergedManualDeploymentRows.map((manualRow, index) => {
    const assignedPeople = activePersonnel.filter((person) => manualRow.personnelIds.includes(person.id));
    return { index: pointDeploymentRows.length + index + 1, point: { id: `manual:${manualRow.id}`, pointCode: `手${index + 1}`, pointName: manualRow.location || "未填寫崗哨位置", color: "purple", pointType: "duty" as const, latitude: 0, longitude: 0 }, postType: manualRow.postType, units: manualRow.unit, count: assignedPeople.length, names: assignedPeople.map((person) => `${person.title}\n${person.name}\n${person.phone}`).join("\n\n") || "—", radios: assignedPeople.map((person) => person.radioCode).join("\n") || "—", equipment: manualRow.equipment, source: "manual" as const, manualRow };
  })];
  const equipmentEditorRow = deploymentRows.find((row) => row.point.id === equipmentEditorPointId);
  const manualPersonnelEditorRow = manualDeploymentRows.find((row) => row.id === manualPersonnelEditorRowId);
  const manualEquipmentEditorRow = [...manualDeploymentRows, ...mergedManualDeploymentRows].find((row) => row.id === manualEquipmentEditorRowId);
  const manualPersonnelCandidates = activePersonnel.filter((person) => (!manualPersonnelKeyword || [person.name, person.personnelCode, person.radioCode].some((value) => value.includes(manualPersonnelKeyword))) && (!manualPersonnelUnit || person.unit === manualPersonnelUnit) && (!manualPersonnelTitle || person.title === manualPersonnelTitle));
  function addManualDeploymentRow() { setManualDeploymentRows((rows) => [...rows, { id: crypto.randomUUID(), postType: "", location: "", unit: "", personnelIds: [], equipment: [] }]); }
  function updateManualDeploymentRow(id: string, changes: Partial<ManualDeploymentRow>) { setManualDeploymentRows((rows) => rows.map((row) => row.id === id ? { ...row, ...changes } : row)); }
  function updateMergedManualDeploymentRow(id: string, changes: Partial<ManualDeploymentRow>) { setMergedManualDeploymentRows((rows) => rows.map((row) => row.id === id ? { ...row, ...changes } : row)); }
  function updateManualEquipmentRow(id: string, changes: Partial<ManualDeploymentRow>) { updateManualDeploymentRow(id, changes); updateMergedManualDeploymentRow(id, changes); }
  function removeManualDeploymentRow(row: ManualDeploymentRow) { if (!window.confirm("刪除會一併移除這列的崗哨、人員與裝備設定。確定繼續？")) return; setManualDeploymentRows((rows) => rows.filter((item) => item.id !== row.id)); }
  function mergeManualDeploymentRow(row: ManualDeploymentRow) { setMergedManualDeploymentRows((rows) => [...rows, row]); setManualDeploymentRows((rows) => rows.filter((item) => item.id !== row.id)); }
  function removeMergedManualDeploymentRow(row: ManualDeploymentRow) { if (!window.confirm("刪除會一併移除這列的崗哨、人員與裝備設定。確定繼續？")) return; setMergedManualDeploymentRows((rows) => rows.filter((item) => item.id !== row.id)); }
  function removePointFromDeploymentTable(point: MapDutyPoint) {
    if (!deploymentRoute || !window.confirm("此操作只會從部署表移除崗位；地圖點位會保留並淡化顯示。確定繼續？")) return;
    setDeploymentPointOverrides((current) => ({ ...current, [deploymentRoute.id]: { addedIds: (current[deploymentRoute.id]?.addedIds ?? []).filter((id) => id !== point.id), excludedIds: [...new Set([...(current[deploymentRoute.id]?.excludedIds ?? []), point.id])] } }));
  }
  function addPointToDeploymentTable() {
    if (!deploymentRoute || !deploymentPointToAddId) return;
    const point = points.find((item) => item.id === deploymentPointToAddId);
    if (!point) return;
    setDeploymentPointOverrides((current) => ({ ...current, [deploymentRoute.id]: { addedIds: [...new Set([...(current[deploymentRoute.id]?.addedIds ?? []), point.id])], excludedIds: (current[deploymentRoute.id]?.excludedIds ?? []).filter((id) => id !== point.id) } }));
    setDeploymentPointToAddId("");
    setMessage(`已將「${point.pointCode}｜${point.pointName}」加入部署表。`);
  }
  async function saveEquipment(pointId: string, selectedItems: string[]) {
    if (!planId) return;
    const previous = deploymentEquipment[pointId] ?? [];
    setDeploymentEquipment((current) => ({ ...current, [pointId]: selectedItems }));
    try {
      await invoke("save_deployment_equipment", { input: { planId, dutyPointId: pointId, selectedItems } });
    } catch (error) {
      setDeploymentEquipment((current) => ({ ...current, [pointId]: previous }));
      setMessage(`無法保存裝備配置：${String(error)}`);
    }
  }
  async function saveWorkspace() {
    if (!planId) { setMessage("請先建立或開啟勤務計畫。 "); return; }
    try {
      await invoke("save_workspace_state", { input: { planId, activeNav, selectedPointId, selectedRouteId, deploymentRouteId, deploymentChoices, mapOutputTitle, mapOutputZoom, mapOutputBearing } });
      setMessage("已儲存目前勤務工作區狀態。 ");
    } catch (error) { setMessage(`無法儲存勤務工作區狀態：${String(error)}`); }
  }
  function workspaceFile(): WorkspaceFile { return { version: 1, planId: planId ?? "", planName, workspace: { activeNav, selectedPointId, selectedRouteId, deploymentRouteId, deploymentChoices, mapOutputTitle, mapOutputZoom, mapOutputBearing } }; }
  async function saveWorkspaceFile() {
    if (!planId) { setMessage("請先建立或開啟勤務計畫。"); return; }
    try {
      const path = await save({ defaultPath: `${planName || "DutyGrid"}-工作區.dutygrid-workspace.json`, filters: [{ name: "DutyGrid 工作區", extensions: ["dutygrid-workspace.json"] }] });
      if (!path) { setMessage("已取消儲存工作區檔案。"); return; }
      await invoke("save_exported_file", { path, bytes: Array.from(new TextEncoder().encode(JSON.stringify(workspaceFile(), null, 2))) });
      setMessage("已儲存工作區檔案；可從 Finder 選取此檔案再次開啟。");
    } catch (error) { setMessage(`無法儲存工作區檔案：${String(error)}`); }
  }
  async function openWorkspaceFile() {
    try {
      const path = await open({ multiple: false, filters: [{ name: "DutyGrid 工作區", extensions: ["dutygrid-workspace.json"] }] });
      if (typeof path !== "string") return;
      const bytes = await invoke<number[]>("read_workspace_file", { path });
      const saved = JSON.parse(new TextDecoder().decode(Uint8Array.from(bytes))) as WorkspaceFile;
      if (saved.version !== 1 || !saved.planId || !saved.workspace) throw new Error("不是可讀取的 DutyGrid 工作區檔案。");
      const plan = dutyPlans.find((item) => item.id === saved.planId);
      if (!plan) { setMessage(`找不到「${saved.planName}」的本機勤務資料；請先在此電腦建立或匯入該勤務計畫。`); return; }
      setPlanId(plan.id); setPlanName(plan.planName);
      setActiveNav(navigation.includes(saved.workspace.activeNav) ? saved.workspace.activeNav : "勤務計畫");
      setSelectedPointId(saved.workspace.selectedPointId);
      setSelectedRouteId(saved.workspace.selectedRouteId);
      setDeploymentRouteId(saved.workspace.deploymentRouteId);
      setDeploymentChoices({});
      setMapOutputTitle(saved.workspace.mapOutputTitle ?? "");
      setMapOutputZoom(saved.workspace.mapOutputZoom ?? 0);
      setMapOutputBearing(saved.workspace.mapOutputBearing ?? 90);
      setMessage(`已開啟「${plan.planName}」工作區檔案。`);
    } catch (error) { setMessage(`無法開啟工作區檔案：${String(error)}`); }
  }
  async function saveGeneratedFile(blob: Blob, fileName: string, filterName: string, extension: string, successMessage: string) {
    const path = await save({ defaultPath: fileName, filters: [{ name: filterName, extensions: [extension] }] });
    if (!path) { setMessage("已取消地圖輸出。 "); return; }
    await invoke("save_exported_file", { path, bytes: Array.from(new Uint8Array(await blob.arrayBuffer())) });
    setMessage(successMessage);
  }
  function exportMapImage() { const image = mapExporter.current?.(); if (!image) { setMessage("地圖尚未完成載入，請稍候再匯出。 "); return; } const anchor = document.createElement("a"); anchor.href = image; anchor.download = `${planName || "DutyGrid"}-勤務地圖.png`; anchor.click(); setMessage("已下載勤務地圖 PNG。 "); }
  async function createMapOutputCanvas() {
    const image = mapOutputExporter.current?.();
    if (!image) { setMessage("輸出地圖尚未完成載入，請稍候再匯出。 "); return null; }
    const mapImage = new Image();
    mapImage.src = image;
    await new Promise<void>((resolve, reject) => { mapImage.onload = () => resolve(); mapImage.onerror = () => reject(new Error("無法產生地圖圖檔。")); });
    const canvas = document.createElement("canvas"); canvas.width = 1754; canvas.height = 1240;
    const context = canvas.getContext("2d"); if (!context) { setMessage("無法建立地圖輸出畫布。 "); return null; }
    context.fillStyle = "#fff"; context.fillRect(0, 0, canvas.width, canvas.height);
    context.strokeStyle = "#111"; context.lineWidth = 5; context.strokeRect(28, 28, canvas.width - 56, canvas.height - 56);
    context.fillStyle = "#111"; context.font = '500 42px "BiauKai", "DFKaiSho-SB", "DFKai-SB", "Kaiti TC", "STKaiti", serif'; context.textAlign = "center";
    context.fillText(mapOutputTitle.trim() || "ＯＯ勤務道路安全維護部署圖", canvas.width / 2, 88);
    context.strokeStyle = "#111"; context.lineWidth = 2; context.strokeRect(52, 116, canvas.width - 104, canvas.height - 168);
    context.drawImage(mapImage, 54, 118, canvas.width - 108, canvas.height - 172);
    const legendRoutes = routes.filter((route) => routeCoordinates(route).length > 1);
    if (legendRoutes.length) { const colors: Record<string, string> = { red: "#df5050", orange: "#ed9a3a", yellow: "#f6c453", green: "#3faf71", blue: "#2d9cdb", purple: "#8966d1" }; const width = 270; const height = 18 + legendRoutes.length * 30; const x = canvas.width - width - 70; const y = canvas.height - height - 68; context.save(); context.fillStyle = "rgb(255 255 255 / 93%)"; context.fillRect(x, y, width, height); context.strokeStyle = "#516170"; context.lineWidth = 1; context.strokeRect(x, y, width, height); context.font = '600 17px "Microsoft JhengHei", sans-serif'; context.textAlign = "left"; legendRoutes.forEach((route, index) => { const lineY = y + 22 + index * 30; const lineStyle = route.lineStyle ?? "solid"; const color = colors[route.color] ?? colors.blue; context.strokeStyle = color; context.lineWidth = 3; context.setLineDash(lineStyle === "dashed" || lineStyle === "dashed_arrow" ? [9, 6] : []); context.beginPath(); context.moveTo(x + 14, lineY); context.lineTo(x + 60, lineY); context.stroke(); context.setLineDash([]); if (lineStyle === "arrow" || lineStyle === "dashed_arrow") { context.fillStyle = color; context.beginPath(); context.moveTo(x + 60, lineY); context.lineTo(x + 52, lineY - 5); context.lineTo(x + 52, lineY + 5); context.closePath(); context.fill(); } context.fillStyle = "#18222d"; context.fillText(route.routeName, x + 72, lineY + 6); }); context.restore(); }
    const northRadians = mapOutputBearing * Math.PI / 180; const northX = Math.sin(northRadians); const northY = -Math.cos(northRadians); const perpendicularX = -northY; const perpendicularY = northX; const arrowBaseX = 8; const arrowBaseY = 4; const arrowTipX = arrowBaseX + northX * 20; const arrowTipY = arrowBaseY + northY * 20;
    context.save(); context.translate(canvas.width - 110, 154); context.fillStyle = "rgb(255 255 255 / 88%)"; context.fillRect(-42, -30, 84, 60); context.strokeStyle = "#17202b"; context.lineWidth = 3; context.beginPath(); context.moveTo(arrowBaseX, arrowBaseY); context.lineTo(arrowTipX, arrowTipY); context.lineTo(arrowTipX - northX * 8 + perpendicularX * 6, arrowTipY - northY * 8 + perpendicularY * 6); context.moveTo(arrowTipX, arrowTipY); context.lineTo(arrowTipX - northX * 8 - perpendicularX * 6, arrowTipY - northY * 8 - perpendicularY * 6); context.stroke(); context.fillStyle = "#17202b"; context.font = '700 22px "BiauKai", "DFKaiSho-SB", "DFKai-SB", "Kaiti TC", "STKaiti", serif'; context.textAlign = "center"; context.fillText("北", -22, 8); context.restore();
    return canvas;
  }
  async function exportMapOutputImage() { try { const canvas = await createMapOutputCanvas(); if (!canvas) return; const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png")); if (!blob) { setMessage("無法建立地圖 PNG。 "); return; } await saveGeneratedFile(blob, `${planName || "DutyGrid"}-道路警衛安全維護部署圖.png`, "PNG 圖片", "png", "已儲存橫向 A4 地圖 PNG。 "); } catch (error) { setMessage(`無法輸出地圖 PNG：${String(error)}`); } }
  async function exportMapOutputPdf() {
    const canvas = await createMapOutputCanvas(); if (!canvas) return;
    const jpeg = Uint8Array.from(atob(canvas.toDataURL("image/jpeg", 0.94).split(",")[1]), (character) => character.charCodeAt(0));
    const encoder = new TextEncoder(); const text = (value: string) => encoder.encode(value); const content = text("q\n841.89 0 0 595.28 0 0 cm\n/Im0 Do\nQ\n");
    const objects = [text("<< /Type /Catalog /Pages 2 0 R >>"), text("<< /Type /Pages /Kids [3 0 R] /Count 1 >>"), text("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 841.89 595.28] /Resources << /XObject << /Im0 5 0 R >> >> /Contents 4 0 R >>"), new Uint8Array([...text(`<< /Length ${content.length} >>\nstream\n`), ...content, ...text("endstream")]), new Uint8Array([...text(`<< /Type /XObject /Subtype /Image /Width ${canvas.width} /Height ${canvas.height} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${jpeg.length} >>\nstream\n`), ...jpeg, ...text("\nendstream")])];
    const header = new Uint8Array([...text("%PDF-1.4\n"), 0xff, 0xff, 0xff, 0xff, ...text("\n")]); const chunks: Uint8Array[] = [header]; const offsets = [0]; let offset = header.length;
    objects.forEach((object, index) => { offsets.push(offset); const wrapped = new Uint8Array([...text(`${index + 1} 0 obj\n`), ...object, ...text("\nendobj\n")]); chunks.push(wrapped); offset += wrapped.length; });
    const xrefOffset = offset; const xref = text(`xref\n0 ${objects.length + 1}\n0000000000 65535 f \n${offsets.slice(1).map((value) => `${String(value).padStart(10, "0")} 00000 n \n`).join("")}trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF`); chunks.push(xref);
    const pdfLength = chunks.reduce((total, chunk) => total + chunk.length, 0); const pdfBuffer = new ArrayBuffer(pdfLength); const pdfBytes = new Uint8Array(pdfBuffer); let pdfOffset = 0; chunks.forEach((chunk) => { pdfBytes.set(chunk, pdfOffset); pdfOffset += chunk.length; });
    try { await saveGeneratedFile(new Blob([pdfBuffer], { type: "application/pdf" }), `${planName || "DutyGrid"}-道路警衛安全維護部署圖.pdf`, "PDF 文件", "pdf", "已儲存整頁橫向 A4 地圖 PDF。 "); } catch (error) { setMessage(`無法輸出地圖 PDF：${String(error)}`); }
  }
  async function exportDeploymentTable() { if (!deploymentRoute) { setMessage("請先選擇要匯出的勤務路線。 "); return; } if (!deploymentRows.length) { setMessage("此路線未連結勤務點位，無法匯出部署表。請以勤務點位建立路線後再匯出。 "); return; } try { const safeRouteName = deploymentRoute.routeName.replace(/[\\/:*?\"<>|]/g, "_"); const fileName = `${planName || "DutyGrid"}-${safeRouteName}-安全維護部署表.xlsx`; const path = await save({ defaultPath: fileName, filters: [{ name: "Excel 活頁簿", extensions: ["xlsx"] }] }); if (!path) { setMessage("已取消匯出安全維護部署表。 "); return; } const bytes = await invoke<number[]>("export_deployment_xlsx", { input: { planName, rows: deploymentRows.map((row) => ({ sequence: row.index, postType: row.postType, pointName: row.point.pointName, unit: row.units, policeCount: row.count, personnelText: row.names === "—" ? "" : row.names, radioText: row.radios === "—" ? "" : row.radios, equipmentText: row.equipment.join("、") })) } }); await invoke("save_exported_file", { path, bytes }); setMessage(`已匯出「${deploymentRoute.routeName}」安全維護部署表。 `); } catch (error) { setMessage(`無法匯出安全維護部署表：${String(error)}`); } }
  return (
    <main className={activeNav === "地圖輸出" ? "app-shell map-output-mode" : activeNav === "部署表" ? "app-shell preview-mode" : "app-shell"}>
      <header className="top-bar">
        <strong className="app-brand"><img alt="勤務人力規劃系統" src={dutyGridIcon} /></strong>
        <span className="plan-name">{planName}</span>
        <span className="top-status" title={activeNav === "人力配置" || activeNav === "地圖輸出" ? message : undefined}>{activeNav === "人力配置" || activeNav === "地圖輸出" ? message : ""}</span>
        <nav className="sidebar" aria-label="主要導覽">
          {navigation.map((item) => <button className={activeNav === item ? "nav-item active" : "nav-item"} key={item} type="button" onClick={() => { setActiveNav(item); if (item !== "點位") { setPendingCoordinate(null); setIsPlacingPoint(false); } if (item !== "路線") { setIsDrawingRoute(false); setManualVertices([]); } if (item === "人力配置") setMessage("請先在地圖點選勤務點位，再從下方篩選並配置人員。"); else if (item === "部署表") setMessage("此預覽對應「安全維護部署表」第一頁的完整欄位；正式匯出將自第 7 列開始填入。點選表格列可定位地圖點位。"); else if (item === "地圖輸出") setMessage("橫向 A4 地圖輸出預覽：西向上；請輸入標題後匯出 PNG。"); else if (item === "路線") setMessage("請選擇已儲存路線，或開始繪製新的勤務路線。"); else if (item === "點位") setMessage("請從地圖右下角選擇圖示，按「開始繪製」後再點擊地圖新增勤務點位。"); else if (item === "路線調整（未實作）") setMessage("此功能頁尚未開始實作。"); }}>{item}</button>)}
        </nav>
        <div className="top-actions">
          <button type="button" disabled={!planId} onClick={() => void saveWorkspaceFile()}>儲存工作區</button>
          {activeNav === "勤務計畫" && <button type="button" onClick={() => void openWorkspaceFile()}>開啟工作區</button>}
          {activeNav === "地圖輸出" && <button type="button" onClick={() => void exportMapOutputImage()}>輸出 PNG</button>}
          {activeNav === "地圖輸出" && <button type="button" onClick={() => void exportMapOutputPdf()}>輸出 PDF</button>}
          {activeNav === "部署表" && <button type="button" onClick={() => void exportDeploymentTable()}>匯出 Excel</button>}
        </div>
      </header>
      <section className="workspace" aria-label="地圖工作區">
        <MapCanvas dimmedPointIds={activeNav === "部署表" ? deploymentPointOverride.excludedIds : []} focusCenter={activeNav === "部署表" ? banqiaoPoliceStation : undefined} isDrawingRoute={isDrawingRoute} manualVertexColor={routeColor} manualVertices={manualVertices} onExportReady={(exporter) => { mapExporter.current = exporter; }} onMapClick={selectPointLocation} onMapPointerMove={(latitude, longitude) => setMapCoordinate({ latitude, longitude })} onPendingCancel={() => { setPendingCoordinate(null); setIsPlacingPoint(false); setMessage("已取消放置暫存點位。"); }} onPointMoved={movePoint} onPointSelect={setSelectedPointId} onRouteVertex={addManualVertex} pendingColor={pendingCoordinate?.color ?? pointColor} pendingCoordinate={pendingCoordinate} pendingPointType={pendingCoordinate?.pointType} personnelLabelPointId={selectedPersonnelLabelPointId} personnelLabels={personnelLabels} points={mapPoints} routeLines={mapRouteLines} selectedPointId={selectedPointId} showPersonnelLabels={showPersonnelLabels} showPointLabels={showPointLabels} />
        <div className="map-label-actions"><button type="button" onClick={() => setShowPointLabels((value) => !value)}>{showPointLabels ? "隱藏標籤" : "顯示標籤"}</button></div>
        {activeNav === "人力配置" && <div className="map-personnel-actions"><button type="button" onClick={() => setShowPersonnelLabels((value) => !value)}>{showPersonnelLabels ? "隱藏人力" : "顯示人力"}</button></div>}
        {activeNav === "路線" && <div className="map-route-controls"><p className="map-route-reminder" role="status">{isDrawingRoute ? "完成後請點選「完成繪製」" : manualVertices.length >= 2 ? "請輸入路線名稱保存路線" : "尚未點選「開始繪製」"}</p><div className="map-route-colors" aria-label="路線顏色">{pointColors.map((color) => <button aria-label={color.label} className={`color-option ${color.value} ${routeColor === color.value ? "selected" : ""}`} key={color.value} type="button" onClick={() => void updateRouteColor(color.value)} />)}</div><div className="map-line-style"><button className={routeLineStyle === "solid" ? "active" : ""} type="button" onClick={() => void updateRouteLineStyle("solid")}>實線</button><button className={routeLineStyle === "dashed" ? "active" : ""} type="button" onClick={() => void updateRouteLineStyle("dashed")}>虛線</button><button className={routeLineStyle === "arrow" ? "active" : ""} type="button" onClick={() => void updateRouteLineStyle("arrow")}>實箭頭線</button><button className={routeLineStyle === "dashed_arrow" ? "active" : ""} type="button" onClick={() => void updateRouteLineStyle("dashed_arrow")}>虛箭頭線</button><button className="route-draw-toggle" type="button" onClick={() => { setIsDrawingRoute((value) => !value); setMessage(isDrawingRoute ? "請輸入路線名稱保存路線。" : "繪製模式：點擊地圖加入折點；完成後請點選「完成繪製」。"); }}>{isDrawingRoute ? "完成繪製" : "開始繪製"}</button></div></div>}
        {activeNav === "部署表" && deploymentRoute && <div className="map-route-legend"><i className={`route-style-icon ${deploymentRoute.color} ${deploymentRoute.lineStyle ?? "solid"}`} /><span>{deploymentRoute.routeName}</span></div>}
        {activeNav === "點位" && <div className="map-point-controls">{pointDrawReminder && !isPlacingPoint && <p className="map-point-reminder" role="alert">尚未點選「開始繪製」，無法繪製</p>}<div className="map-point-options" aria-label="點位圖示與顏色">{pointColors.map((color) => <button aria-label={color.label} className={`color-option ${color.value} ${pointType === "duty" && pointColor === color.value ? "selected" : ""}`} key={color.value} type="button" onClick={() => { setPointType("duty"); setPointColor(color.value); }} />)}<button aria-label="空心圓圈" className={pointType === "hollow" ? "point-symbol hollow selected" : "point-symbol hollow"} type="button" onClick={() => setPointType("hollow")}>○</button><button aria-label="號誌圖案" className={pointType === "signal" ? "point-symbol signal selected" : "point-symbol signal"} type="button" onClick={() => setPointType("signal")}>🚦</button></div><div className="map-point-actions"><button className={isPlacingPoint ? "active" : ""} type="button" onClick={() => { const next = !isPlacingPoint; setIsPlacingPoint(next); setPointDrawReminder(false); setMessage(next ? "點位繪製模式：請點擊地圖放置點位。" : "已取消新增點位繪製。"); }}>{isPlacingPoint ? "取消繪製" : "開始繪製"}</button></div></div>}
      </section>
      <aside className={activeNav === "路線" ? "inspector route-inspector" : activeNav === "人力配置" ? "inspector personnel-inspector" : "inspector"} aria-label="詳細資料面板" onClick={(event) => { if (event.target === event.currentTarget) { setSelectedPointId(null); setSelectedRouteId(null); } }}>
        <h1>{activeNav === "點位" ? "勤務點位" : activeNav === "路線" ? "勤務路線" : activeNav === "人力配置" ? "人力配置" : activeNav === "部署表" ? "安全維護部署表預覽" : activeNav === "地圖輸出" ? "地圖輸出預覽" : activeNav === "路線調整（未實作）" ? "路線調整（未實作）" : planId ? "勤務計畫" : "開始建立勤務計畫"}</h1>
        {activeNav !== "部署表" && activeNav !== "地圖輸出" && activeNav !== "人力配置" && <p>{message}</p>}
        {validationError && <p className="validation-error" role="alert">{validationError}</p>}
        {!planId && activeNav === "勤務計畫" && <><form onSubmit={(event) => { event.preventDefault(); void createPlan(); }}><label className="field-label" htmlFor="plan-name">勤務計畫名稱</label><input id="plan-name" value={newPlanName} onChange={(event) => { setNewPlanName(event.target.value); setPlanName(event.target.value.trim() || "尚未開啟勤務計畫"); }} /><button type="submit">新增勤務計畫</button></form><section className="saved-workspaces"><strong>開啟既有工作區</strong><span>從 Finder 選取先前儲存的工作區檔案。</span><button type="button" onClick={() => void openWorkspaceFile()}>選取工作區檔案</button></section></>}
        {planId && activeNav === "勤務計畫" && <section className="plan-summary"><span>目前開啟</span><strong>{planName}</strong><p>請由頂部導覽選擇「點位」以新增、查看或刪除勤務點位。</p><button className="secondary-button" type="button" onClick={exitPlan}>退出勤務計畫</button></section>}
        {activeNav === "點位" && <><div className="point-list">{points.length ? points.map((point) => <div className="point-list-row" key={point.id}><button className={selectedPointId === point.id ? "point-list-item selected" : "point-list-item"} type="button" onClick={() => setSelectedPointId(point.id)}><i className={point.pointType === "signal" ? "signal-point-chip" : `point-color-chip ${point.pointType === "hollow" ? "hollow" : point.color}`} />{point.pointType === "signal" ? "號誌｜" : point.pointType === "hollow" ? "空心｜" : ""}{point.pointCode}｜{point.pointName}</button><button className="delete-button" type="button" onClick={() => setPendingDelete(point)}>刪除</button></div>) : <p>尚無勤務點位。請按地圖右下角「開始繪製」後再點擊地圖新增。</p>}</div>{pendingCoordinate && <form onSubmit={(event) => { event.preventDefault(); void createPoint(); }}><h2>新增點位</h2><p>{pendingCoordinate.latitude.toFixed(5)}, {pendingCoordinate.longitude.toFixed(5)}</p><label className="field-label">點位類型</label><p>{pendingCoordinate.pointType === "signal" ? "號誌點位（可配置燈控人員）" : pendingCoordinate.pointType === "hollow" ? "空心圓圈點位" : "一般勤務點位"}</p><label className="field-label">點位編號</label><input value={pointCode} onChange={(event) => setPointCode(event.target.value)} /><label className="field-label">位置名稱</label><input value={pointName} onChange={(event) => setPointName(event.target.value)} /><button type="submit">保存{pendingCoordinate.pointType === "signal" ? "號誌" : "勤務"}點位</button></form>}</>}
        {activeNav === "路線" && <>
          <p>按「開始繪製」後，依序點擊地圖：兩點即成直線，繼續點擊可新增折點。空白鍵復原最後一點。</p>
          {routes.length > 0 && <><span className="section-caption">已儲存路線</span><div className="route-list">{routes.map((route) => <div className={selectedRouteId === route.id ? "route-list-row selected" : "route-list-row"} key={route.id}><button className="route-list-select" type="button" onClick={() => { setSelectedRouteId(route.id); setRouteColor(route.color); setRouteLineStyle(route.lineStyle ?? "solid"); setMessage(`已選取路線 ${route.routeName}；可由地圖控制變更顏色與線型。`); }}><i className={`route-style-icon ${route.color} ${route.lineStyle ?? "solid"}`} />{route.routeName}</button><button className="route-delete-button" aria-label={`刪除 ${route.routeName}`} type="button" onClick={() => void deleteRoute(route)}>×</button></div>)}</div>{selectedRouteId && <button className="save-common-route" type="button" onClick={() => void saveSelectedRouteAsCommon()}>儲存為常用路線</button>}</>}
          <form onSubmit={(event) => { event.preventDefault(); void saveManualRoute(); }}><label className="field-label" htmlFor="route-name">路線名稱</label><div className="route-name-row"><input aria-describedby="route-name-error" aria-invalid={validationError === "請輸入路線名稱。"} id="route-name" value={routeName} onChange={(event) => { setRouteName(event.target.value); if (validationError === "請輸入路線名稱。") setValidationError(""); }} /><button type="submit">保存</button></div>{validationError === "請輸入路線名稱。" && <p className="route-name-error" id="route-name-error" role="alert">{validationError}</p>}<label className="field-label">路線顏色</label><div className="color-options">{pointColors.map((color) => <button aria-label={color.label} className={`color-option ${color.value} ${routeColor === color.value ? "selected" : ""}`} key={color.value} type="button" onClick={() => setRouteColor(color.value)} />)}</div><div className="drawing-actions"><button type="button" onClick={() => { setIsDrawingRoute((value) => !value); setMessage(isDrawingRoute ? "請輸入路線名稱保存路線。" : "繪製模式：點擊地圖加入折點；完成後請點選「完成繪製」。"); }}>{isDrawingRoute ? "完成繪製" : "開始繪製"}</button></div></form>
          <section className="point-route-placeholder"><strong>依點位建立路線</strong><span>功能尚未實作</span></section>
          <section className="common-route-history"><strong>已儲存歷史路線</strong>{commonRoutes.length ? <div className="route-list">{commonRoutes.map((route) => <div className="route-list-row" key={route.id}><button className="route-list-select" type="button" onClick={() => void applyCommonRoute(route)}><i className={`point-color-chip ${route.color}`} />{route.routeName}</button><button className="route-delete-button" aria-label={`刪除常用路線 ${route.routeName}`} type="button" onClick={() => void deleteCommonRoute(route)}>×</button></div>)}</div> : <span>尚無常用路線。</span>}</section>
        </>}
        {activeNav === "人力配置" && <><p>{selectedPoint ? `目前配置點位：${selectedPoint.pointCode}｜${selectedPoint.pointName}` : "請先在地圖點選勤務點位，再從下方篩選並配置人員。"}</p><section className="personnel-import"><strong>匯入人力資料</strong><span>接受 .csv 或 .xlsx。必填欄位：personnel_code、radio_code、name、title、unit、phone。</span><div className="personnel-import-actions"><button type="button" onClick={() => void choosePersonnelFile()}>選擇人力檔案</button><button type="button" onClick={() => void importDefaultPersonnelFile()}>讀取測試資料集</button></div><small>從 data/ 讀取；優先 .xlsx，其次 .csv。</small></section><div className="personnel-filters"><input aria-label="搜尋人員" placeholder="搜尋姓名、警號或無線電代號" value={personnelKeyword} onChange={(event) => setPersonnelKeyword(event.target.value)} /><select aria-label="篩選單位" value={personnelUnit} onChange={(event) => setPersonnelUnit(event.target.value)}><option value="">所有單位</option>{units.map((unit) => <option key={unit} value={unit}>{unit} 單位</option>)}</select><select aria-label="篩選職稱" value={personnelTitle} onChange={(event) => setPersonnelTitle(event.target.value)}><option value="">所有職稱</option>{titles.map((title) => <option key={title} value={title}>{title}</option>)}</select></div>{selectedPoint && <section className="assigned-personnel"><strong>已配置人員（{pointAssignments.length}）</strong>{pointAssignments.length ? pointAssignments.map((assignment) => { const person = personnel.find((item) => item.id === assignment.personnelId); return <div className="assignment-row" key={assignment.id}><span>{person?.name ?? "已刪除人員"} · {assignment.assignedTitle}</span><button type="button" onClick={() => void removePersonnelAssignment(assignment)}>移除</button></div>; }) : <span>尚未配置人員。</span>}</section>}<section className="personnel-results"><strong>可配置人員（{prioritizedPersonnel.length}）</strong><div className="personnel-scroll">{prioritizedPersonnel.map((person) => { const assignedAtPoint = personnelAssignments.some((assignment) => assignment.personnelId === person.id && assignment.dutyPointId === selectedPointId); const preferredAssignment = sameCodeAssignments.find((assignment) => assignment.personnelId === assignment.personnelId && assignment.dutyPointId === selectedPointId); const preferredPoint = points.find((point) => point.id === preferredAssignment?.dutyPointId); return <div className={preferredPersonnelIds.has(person.id) ? "personnel-row preferred" : "personnel-row"} key={person.id}><span><b>{person.name}{preferredPoint && <em>已配置於 {preferredPoint.color} 色 {preferredPoint.pointCode}</em>}</b><small>{person.personnelCode} · {person.radioCode} · {person.unit} 單位 · {person.title}</small></span><button disabled={!selectedPoint || assignedAtPoint} type="button" onClick={() => void assignPersonnel(person)}>{assignedAtPoint ? "已配置" : "配置"}</button></div>; })}</div></section></>}
        {activeNav === "人力配置" && personnelImportLog && <section className="personnel-import-log"><details open={personnelImportLog.rejectedRows > 0}><summary>最近匯入：{personnelImportLog.sourceFileName}｜成功 {personnelImportLog.acceptedRows}/{personnelImportLog.totalRows}，拒絕 {personnelImportLog.rejectedRows}</summary>{personnelImportLog.errors.length > 0 ? <ol>{personnelImportLog.errors.map((error) => <li key={error.rowNumber}>第 {error.rowNumber} 列：{error.errorReason}<code>{error.rawRowJson}</code></li>)}</ol> : <span>沒有拒絕紀錄。</span>}</details></section>}
        {activeNav === "部署表" && <section className="deployment-preview">
          <div className="deployment-export-actions"><button type="button" onClick={exportMapImage}>匯出地圖 PNG</button><button type="button" onClick={() => void exportDeploymentTable()}>匯出安全維護部署表</button></div>
          {deploymentRoute && <section className="deployment-table-tools"><label>新增地圖點位<select aria-label="新增地圖點位" value={deploymentPointToAddId} onChange={(event) => setDeploymentPointToAddId(event.target.value)}><option value="">請選擇點位</option>{points.filter((point) => !deploymentPoints.some((item) => item.id === point.id)).map((point) => <option key={point.id} value={point.id}>{point.pointCode}｜{point.pointName}</option>)}</select></label><button type="button" disabled={!deploymentPointToAddId} onClick={addPointToDeploymentTable}>加入部署表</button></section>}
          <div className="deployment-route-tabs" role="tablist" aria-label="勤務路線部署表">
            {routes.length ? routes.map((route) => <button aria-selected={deploymentRouteId === route.id} className={deploymentRouteId === route.id ? "deployment-route-tab active" : "deployment-route-tab"} key={route.id} role="tab" type="button" onClick={() => { setDeploymentRouteId(route.id); setSelectedPointId(null); setSelectedPersonnelLabelPointId(null); setEquipmentEditorPointId(null); }}>{route.routeName}</button>) : <span className="deployment-route-empty">尚無已儲存路線；請先至「路線」建立勤務路線。</span>}
          </div>
          <p className="deployment-workspace-note">{deploymentRoute ? `目前檢視「${deploymentRoute.routeName}」：地圖與部署表僅顯示此路線的點位。點選表格列可定位地圖點位。` : "請先從上方分頁選擇要檢視的勤務路線。"}</p>
          <div className="a4-preview" onClick={(event) => { if (event.target === event.currentTarget) { setSelectedPointId(null); setSelectedPersonnelLabelPointId(null); setEquipmentEditorPointId(null); } }}>
            <div className="deployment-sheet">
              <h2>新北市道路警衛區ＸＸ道路警衛段<br />（ＸＸ演習）蒞臨場所警衛安全維護部署表</h2>
              <div className="deployment-brief"><p>勤務時段：115年Ｏ月Ｏ日Ｏ時至Ｏ時。</p><p>各監巡區、（分）隊勤教時間：自行勤教。（檢查應勤裝備、交付任務）</p><p>本段勤教時、地：Ｏ時Ｏ分（ＸＸＸＸ）。（Ｏ時Ｏ分就座完畢）</p><p>場檢時間：Ｏ時Ｏ分。</p><p>重點部署時間：Ｏ時Ｏ分【ＸＸＸＸ】</p><p>全員部署時間：Ｏ時Ｏ分。</p></div>
              {deploymentRoute?.routeType === "manual" && !deploymentRows.length && <p className="deployment-route-warning">此手繪路線附近尚未找到勤務點位；請在路線附近新增點位後再檢視部署表。</p>}
              {!deploymentRoute && <p className="deployment-route-warning">請先從上方分頁選擇勤務路線。</p>}
              <table><thead><tr><th>編號</th><th>崗哨別</th><th>崗哨位置</th><th>派遣單位</th><th>警力</th><th>職稱姓名</th><th>無線電代號</th><th>服裝及應勤裝備</th><th>分（協調）區協調員電話</th></tr></thead><tbody>{deploymentRows.map((row) => {
                const isManual = row.source === "manual";
                return <tr className={!isManual && selectedPointId === row.point.id ? "selected" : ""} key={row.point.id} onClick={() => {
                  if (isManual) return;
                  setSelectedPointId(row.point.id); setShowPersonnelLabels(false); setSelectedPersonnelLabelPointId(row.point.id);
                  setMessage(`已定位勤務點位：${row.point.pointCode}｜${row.point.pointName}，並顯示配置人員。`);
                }}><td><button aria-label={`刪除部署表第 ${row.index} 列`} className="delete-button deployment-external-delete" title="刪除這列" type="button" onClick={(event) => { event.stopPropagation(); if (row.manualRow) removeMergedManualDeploymentRow(row.manualRow); else removePointFromDeploymentTable(row.point); }}>×</button>{row.index}</td><td><select value={row.postType} onChange={(event) => {
                  if (row.manualRow) updateMergedManualDeploymentRow(row.manualRow.id, { postType: event.target.value });
                  else setDeploymentChoices((current) => ({ ...current, [row.point.id]: { postType: event.target.value, unit: current[row.point.id]?.unit ?? row.units } }));
                }}><option value="">請選擇</option>{deploymentPostTypes.map((value) => <option key={value} value={value}>{value}</option>)}</select></td><td>{row.manualRow ? <input aria-label="手動崗哨位置" value={row.manualRow.location} onChange={(event) => updateMergedManualDeploymentRow(row.manualRow!.id, { location: event.target.value })} /> : row.point.pointName}</td><td><select value={row.units} onChange={(event) => {
                  if (row.manualRow) updateMergedManualDeploymentRow(row.manualRow.id, { unit: event.target.value });
                  else setDeploymentChoices((current) => ({ ...current, [row.point.id]: { postType: current[row.point.id]?.postType ?? row.postType, unit: event.target.value } }));
                }}><option value="">請選擇</option>{deploymentUnits.map((value) => <option key={value} value={value}>{value}</option>)}</select></td><td>{row.count}</td><td>{row.names}</td><td>{row.radios}</td><td className="equipment-cell">{row.manualRow ? <button type="button" onClick={(event) => { event.stopPropagation(); setManualEquipmentEditorRowId(row.manualRow!.id); }}>{row.equipment.length ? row.equipment.join("、") : "選擇裝備"}</button> : <button type="button" onClick={(event) => { event.stopPropagation(); setSelectedPointId(row.point.id); setShowPersonnelLabels(false); setSelectedPersonnelLabelPointId(row.point.id); setEquipmentEditorPointId(row.point.id); }}>{row.equipment.length ? row.equipment.join("、") : "選擇裝備"}</button>}</td><td>—</td></tr>;
              })}</tbody></table>
            </div>
          </div>
          {manualPersonnelEditorRow && <div className="equipment-backdrop" role="dialog" aria-modal="true" aria-label="選擇手動崗哨人員" onClick={() => setManualPersonnelEditorRowId(null)}><section className="equipment-dialog personnel-picker" onClick={(event) => event.stopPropagation()}><div className="equipment-dialog-title"><div><strong>選擇勤務人員</strong><span>{manualPersonnelEditorRow.location || "未填寫崗哨位置"}</span></div><button type="button" aria-label="關閉人員選擇" onClick={() => setManualPersonnelEditorRowId(null)}>×</button></div><div className="personnel-filters"><input aria-label="搜尋人員" placeholder="搜尋姓名、警號或無線電代號" value={manualPersonnelKeyword} onChange={(event) => setManualPersonnelKeyword(event.target.value)} /><select aria-label="篩選單位" value={manualPersonnelUnit} onChange={(event) => setManualPersonnelUnit(event.target.value)}><option value="">所有單位</option>{units.map((unit) => <option key={unit} value={unit}>{unit}</option>)}</select><select aria-label="篩選職稱" value={manualPersonnelTitle} onChange={(event) => setManualPersonnelTitle(event.target.value)}><option value="">所有職稱</option>{titles.map((title) => <option key={title} value={title}>{title}</option>)}</select></div><div className="equipment-summary">已選 {manualPersonnelEditorRow.personnelIds.length} 人</div><div className="equipment-options">{manualPersonnelCandidates.map((person) => { const checked = manualPersonnelEditorRow.personnelIds.includes(person.id); return <label key={person.id}><input type="checkbox" checked={checked} onChange={() => updateManualDeploymentRow(manualPersonnelEditorRow.id, { personnelIds: checked ? manualPersonnelEditorRow.personnelIds.filter((id) => id !== person.id) : [...manualPersonnelEditorRow.personnelIds, person.id] })} />{person.name}｜{person.personnelCode}｜{person.radioCode}｜{person.unit}｜{person.title}</label>; })}</div><div className="equipment-dialog-actions"><button type="button" onClick={() => updateManualDeploymentRow(manualPersonnelEditorRow.id, { personnelIds: [] })}>清除</button><button type="button" onClick={() => setManualPersonnelEditorRowId(null)}>完成</button></div></section></div>}
          {equipmentEditorRow && <div className="equipment-backdrop" role="dialog" aria-modal="true" aria-label="選擇服裝及應勤裝備" onClick={() => setEquipmentEditorPointId(null)}><section className="equipment-dialog" onClick={(event) => event.stopPropagation()}><div className="equipment-dialog-title"><div><strong>服裝及應勤裝備</strong><span>{equipmentEditorRow.point.pointCode}｜{equipmentEditorRow.point.pointName}</span></div><button type="button" aria-label="關閉裝備選擇" onClick={() => setEquipmentEditorPointId(null)}>×</button></div><label>快速套用<select value="" onChange={(event) => { const preset = equipmentPresets.find((item) => item.name === event.target.value); if (preset) void saveEquipment(equipmentEditorRow.point.id, preset.items); event.currentTarget.value = ""; }}><option value="">選擇常用勤務組合</option>{equipmentPresets.map((preset) => <option key={preset.name} value={preset.name}>{preset.name}（{preset.items.length} 項）</option>)}</select></label><div className="equipment-summary">已選 {equipmentEditorRow.equipment.length} 項：{equipmentEditorRow.equipment.join("、") || "尚未選擇"}</div><div className="equipment-options">{equipmentItems.map((item) => { const checked = equipmentEditorRow.equipment.includes(item); return <label key={item}><input type="checkbox" checked={checked} onChange={() => void saveEquipment(equipmentEditorRow.point.id, checked ? equipmentEditorRow.equipment.filter((value) => value !== item) : [...equipmentEditorRow.equipment, item])} />{item}</label>; })}</div><div className="equipment-dialog-actions"><button type="button" onClick={() => void saveEquipment(equipmentEditorRow.point.id, [])}>清除</button><button type="button" onClick={() => setEquipmentEditorPointId(null)}>完成</button></div></section></div>}
          <section className="manual-deployment manual-deployment-v2"><div><strong>手動新增崗哨</strong><button type="button" onClick={addManualDeploymentRow}>新增欄位</button></div>{manualDeploymentRows.length > 0 && <table><thead><tr><th>編號</th><th>崗哨別</th><th>崗哨位置</th><th>派遣單位</th><th>警力</th><th>職稱姓名</th><th>無線電代號</th><th>服裝及應勤裝備</th><th>操作</th></tr></thead><tbody>{manualDeploymentRows.map((row, index) => { const people = activePersonnel.filter((person) => row.personnelIds.includes(person.id)); return <tr key={row.id}><td>手{index + 1}</td><td><select value={row.postType} onChange={(event) => updateManualDeploymentRow(row.id, { postType: event.target.value })}><option value="">請選擇</option>{deploymentPostTypes.map((value) => <option key={value} value={value}>{value}</option>)}</select></td><td><input value={row.location} onChange={(event) => updateManualDeploymentRow(row.id, { location: event.target.value })} /></td><td><select value={row.unit} onChange={(event) => updateManualDeploymentRow(row.id, { unit: event.target.value })}><option value="">請選擇</option>{deploymentUnits.map((value) => <option key={value} value={value}>{value}</option>)}</select></td><td>{people.length}</td><td><button type="button" onClick={() => setManualPersonnelEditorRowId(row.id)}>選擇人力（{people.length}）</button></td><td>{people.map((person) => person.radioCode).join("、") || "—"}</td><td className="equipment-cell"><button type="button" onClick={() => setManualEquipmentEditorRowId(row.id)}>{row.equipment.length ? row.equipment.join("、") : "選擇裝備"}</button></td><td><button type="button" onClick={() => mergeManualDeploymentRow(row)}>↑ 合併到上表</button><button className="delete-button" style={{ backgroundColor: "#df5050", color: "#fff" }} type="button" onClick={() => removeManualDeploymentRow(row)}>刪除</button></td></tr>; })}</tbody></table>}</section>
        </section>}
        {activeNav === "地圖輸出" && <section className="map-output-preview"><div className="map-output-sheet"><div className="map-output-title"><input aria-label="地圖標題" placeholder="ＯＯ勤務道路安全維護部署圖" value={mapOutputTitle} onChange={(event) => setMapOutputTitle(event.target.value)} /></div><div className="map-output-frame"><MapCanvas bearing={90} fitToData interactive isDrawingRoute={false} manualVertexColor="blue" manualVertices={[]} onBearingChange={setMapOutputBearing} onExportReady={(exporter) => { mapOutputExporter.current = exporter; }} onMapClick={() => {}} onPendingCancel={() => {}} onPointMoved={() => {}} onPointSelect={() => {}} onRouteVertex={() => {}} pendingColor="red" pendingCoordinate={null} personnelLabelPointId={null} personnelLabels={{}} points={points} routeLines={routes.map((route) => ({ color: route.color, dashed: route.lineStyle === "dashed" || route.lineStyle === "dashed_arrow", arrow: route.lineStyle === "arrow" || route.lineStyle === "dashed_arrow", coordinates: routeCoordinates(route) })).filter((route) => route.coordinates.length > 1)} selectedPointId={null} showNavigation={false} showPersonnelLabels={false} showPointLabels zoomAdjustment={mapOutputZoom} /><div className="map-north-indicator" aria-label="北向指示">北 <span style={{ transform: `rotate(${mapOutputBearing}deg)` }}>↑</span></div></div><table className="map-police-summary" aria-label="警力統計表格預留"><caption>警力統計（預留）</caption><thead><tr><th>單位</th><th>編制</th><th>實到</th></tr></thead><tbody><tr><td colSpan={3}>後續實作</td></tr></tbody></table></div><div className="map-output-actions"><label>地圖比例<input aria-label="調整地圖比例" max="3" min="-3" step="0.25" type="range" value={mapOutputZoom} onChange={(event) => setMapOutputZoom(Number(event.target.value))} /><span>{mapOutputZoom === 0 ? "自動範圍" : `${mapOutputZoom > 0 ? "放大" : "縮小"} ${Math.abs(mapOutputZoom).toFixed(2)}`}</span></label><div><span>橫向 A4・西向上</span><button type="button" onClick={() => setMapOutputZoom(0)}>重設比例</button></div></div></section>}
        {activeNav === "地圖輸出" && <table className="map-output-police-summary" aria-label="警力配置表預留"><thead><tr><th>崗哨別</th><th>圖示</th><th>數量</th></tr></thead><tbody>{Array.from({ length: 5 }, (_, index) => <tr key={index}><td>&nbsp;</td><td>&nbsp;</td><td>&nbsp;</td></tr>)}</tbody></table>}
        {activeNav === "路線調整（未實作）" && <p>此分頁預留給後續路線調整功能，目前尚未實作。</p>}
      </aside>
      {manualEquipmentEditorRow && <div className="equipment-backdrop" role="dialog" aria-modal="true" aria-label="選擇手動崗哨裝備" onClick={() => setManualEquipmentEditorRowId(null)}><section className="equipment-dialog" onClick={(event) => event.stopPropagation()}><div className="equipment-dialog-title"><div><strong>服裝及應勤裝備</strong><span>{manualEquipmentEditorRow.location || "手動崗哨"}</span></div><button type="button" aria-label="關閉裝備選擇" onClick={() => setManualEquipmentEditorRowId(null)}>×</button></div><label>快速套用<select value="" onChange={(event) => { const preset = equipmentPresets.find((item) => item.name === event.target.value); if (preset) updateManualEquipmentRow(manualEquipmentEditorRow.id, { equipment: preset.items }); event.currentTarget.value = ""; }}><option value="">選擇常用勤務組合</option>{equipmentPresets.map((preset) => <option key={preset.name} value={preset.name}>{preset.name}（{preset.items.length} 項）</option>)}</select></label><div className="equipment-summary">已選 {manualEquipmentEditorRow.equipment.length} 項：{manualEquipmentEditorRow.equipment.join("、") || "尚未選擇"}</div><div className="equipment-options">{equipmentItems.map((item) => <label key={item}><input type="checkbox" checked={manualEquipmentEditorRow.equipment.includes(item)} onChange={() => updateManualEquipmentRow(manualEquipmentEditorRow.id, { equipment: manualEquipmentEditorRow.equipment.includes(item) ? manualEquipmentEditorRow.equipment.filter((value) => value !== item) : [...manualEquipmentEditorRow.equipment, item] })} />{item}</label>)}</div><div className="equipment-dialog-actions"><button type="button" onClick={() => updateManualEquipmentRow(manualEquipmentEditorRow.id, { equipment: [] })}>清除</button><button type="button" onClick={() => setManualEquipmentEditorRowId(null)}>完成</button></div></section></div>}
      <footer className="status-bar"><span>資料庫：尚未初始化　｜　路口參考：已隨 App 提供</span><span className="map-coordinate">座標：{mapCoordinate ? `經度 ${mapCoordinate.longitude.toFixed(6)}，緯度 ${mapCoordinate.latitude.toFixed(6)}` : "將游標移到地圖上顯示經緯度"}</span><span>© 1136023. All rights reserved</span></footer>
      {pendingDelete && <div className="confirm-backdrop" role="dialog" aria-modal="true"><section className="confirm-dialog"><h2>刪除勤務點位？</h2><p>刪除會一併移除相關點位、路線關聯、人力配置與裝備設定。確定繼續？</p><p>目標：「{pendingDelete.pointCode}｜{pendingDelete.pointName}」。</p><div><button type="button" onClick={() => setPendingDelete(null)}>取消</button><button className="delete-button" type="button" onClick={() => void deletePoint()}>確認刪除</button></div></section></div>}
    </main>
  );
}
