import { useEffect, useRef } from "react";
import { Map, Marker, NavigationControl, type Map as MapLibreMap, type StyleSpecification } from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";

const banqiaoCenter: [number, number] = [121.4615, 25.0097];
function basemapStyle(tiles: string[], attribution: string): StyleSpecification { return {
  version: 8,
  sources: { basemap: { type: "raster", tiles, tileSize: 256, maxzoom: 19, attribution } },
  layers: [{ id: "basemap", type: "raster", source: "basemap", paint: { "raster-saturation": 0, "raster-contrast": 0, "raster-brightness-min": 0, "raster-brightness-max": 1 } }],
}; }
const nlscBasemapStyle = basemapStyle(["https://wmts.nlsc.gov.tw/wmts/EMAP/default/GoogleMapsCompatible/{z}/{y}/{x}"], "© 國土測繪中心");
const fallbackBasemapStyle = basemapStyle(["https://tile.openstreetmap.org/{z}/{x}/{y}.png"], "© OpenStreetMap contributors");

export type MapDutyPoint = { id: string; pointCode: string; pointName: string; color: string; pointType: "duty" | "hollow" | "signal"; latitude: number; longitude: number };
export function MapCanvas({ bearing = 0, fitToData = false, focusCenter, interactive = true, isDrawingRoute, manualVertexColor, manualVertices, onBearingChange, onExportReady, onMapClick, onMapPointerMove, onPendingCancel, onPointMoved, onPointSelect, onRouteVertex, pendingColor, pendingCoordinate, pendingPointType = "duty", personnelLabelPointId, personnelLabels, points, routeLines, selectedPointId, showNavigation = true, showPersonnelLabels, showPointLabels, zoomAdjustment = 0 }: { bearing?: number; fitToData?: boolean; focusCenter?: [number, number]; interactive?: boolean; isDrawingRoute: boolean; manualVertexColor: string; manualVertices: [number, number][]; onBearingChange?: (bearing: number) => void; onExportReady: (exporter: () => string | null) => void; onMapClick: (latitude: number, longitude: number) => void; onMapPointerMove?: (latitude: number, longitude: number) => void; onPendingCancel: () => void; onPointMoved: (point: MapDutyPoint, latitude: number, longitude: number) => void; onPointSelect: (pointId: string) => void; onRouteVertex: (latitude: number, longitude: number) => void; pendingColor: string; pendingCoordinate: { latitude: number; longitude: number } | null; pendingPointType?: MapDutyPoint["pointType"]; personnelLabelPointId: string | null; personnelLabels: Record<string, string[]>; points: MapDutyPoint[]; routeLines: { color: string; coordinates: [number, number][]; dashed?: boolean; arrow?: boolean; opacity?: number }[]; selectedPointId: string | null; showNavigation?: boolean; showPersonnelLabels: boolean; showPointLabels: boolean; zoomAdjustment?: number }) {
  const container = useRef<HTMLDivElement>(null);
  const map = useRef<MapLibreMap | null>(null);
  const onMapClickRef = useRef(onMapClick);
  const onRouteVertexRef = useRef(onRouteVertex);
  const onBearingChangeRef = useRef(onBearingChange);
  const onMapPointerMoveRef = useRef(onMapPointerMove);
  const isDrawingRouteRef = useRef(isDrawingRoute);
  const hasFallback = useRef(false);
  const fittedZoom = useRef<number | null>(null);
  const fitDataKey = JSON.stringify({ points: points.map((point) => [point.id, point.longitude, point.latitude]), routes: routeLines.map((route) => route.coordinates) });

  useEffect(() => { onMapClickRef.current = onMapClick; }, [onMapClick]);
  useEffect(() => { onBearingChangeRef.current = onBearingChange; }, [onBearingChange]);
  useEffect(() => { onMapPointerMoveRef.current = onMapPointerMove; }, [onMapPointerMove]);
  useEffect(() => { onExportReady(() => {
    const activeMap = map.current;
    if (!activeMap) return null;
    const source = activeMap.getCanvas();
    const canvas = document.createElement("canvas");
    canvas.width = source.width;
    canvas.height = source.height;
    const context = canvas.getContext("2d");
    if (!context) return null;
    try { context.drawImage(source, 0, 0); } catch { return null; }
    const scale = source.width / Math.max(source.clientWidth, 1);
    const colors: Record<string, string> = { red: "#df5050", orange: "#ed9a3a", yellow: "#f6c453", green: "#3faf71", blue: "#2d9cdb", purple: "#8966d1" };
    context.scale(scale, scale);
    routeLines.forEach((route) => { const pixels = route.coordinates.map((coordinate) => activeMap.project(coordinate)); context.beginPath(); pixels.forEach((pixel, index) => { if (index === 0) context.moveTo(pixel.x, pixel.y); else context.lineTo(pixel.x, pixel.y); }); context.strokeStyle = colors[route.color] ?? colors.blue; context.globalAlpha = route.opacity ?? 1; context.lineWidth = 2.5; context.lineCap = "round"; if (route.dashed) context.setLineDash([10, 8]); context.stroke(); context.setLineDash([]); if (route.arrow && pixels.length > 1) { const tip = pixels[pixels.length - 1]; const previous = pixels[pixels.length - 2]; const angle = Math.atan2(tip.y - previous.y, tip.x - previous.x); context.fillStyle = colors[route.color] ?? colors.blue; context.beginPath(); context.moveTo(tip.x, tip.y); context.lineTo(tip.x - Math.cos(angle - Math.PI / 6) * 10, tip.y - Math.sin(angle - Math.PI / 6) * 10); context.lineTo(tip.x - Math.cos(angle + Math.PI / 6) * 10, tip.y - Math.sin(angle + Math.PI / 6) * 10); context.closePath(); context.fill(); } });
    context.globalAlpha = 1;
    points.forEach((point) => { const pixel = activeMap.project([point.longitude, point.latitude]); const fill = point.color === "red" ? "#df5050" : point.color === "orange" ? "#ed9a3a" : point.color === "yellow" ? "#f6c453" : point.color === "green" ? "#3faf71" : point.color === "purple" ? "#8966d1" : "#2d9cdb"; context.save(); context.globalAlpha = 1; context.strokeStyle = "#243242"; context.lineWidth = 2; if (point.pointType === "signal") { context.fillStyle = "#23313e"; context.roundRect(pixel.x - 7, pixel.y - 12, 14, 24, 3); context.fill(); [[-6, "#ef5a58"], [0, "#f4c44e"], [6, "#48b879"]].forEach(([offset, color]) => { context.beginPath(); context.fillStyle = color as string; context.arc(pixel.x, pixel.y + (offset as number), 2, 0, Math.PI * 2); context.fill(); }); } else { context.beginPath(); context.fillStyle = point.pointType === "hollow" ? "#fff" : fill; context.arc(pixel.x, pixel.y, 8, 0, Math.PI * 2); context.fill(); context.strokeStyle = point.pointType === "hollow" ? "#2d9cdb" : "#243242"; context.lineWidth = point.pointType === "hollow" ? 3 : 2; context.stroke(); } context.font = '700 14px "BiauKai", "DFKaiSho-SB", "DFKai-SB", "Kaiti TC", "STKaiti", serif'; context.fillStyle = "#18222d"; context.strokeStyle = "#ffffff"; context.lineWidth = 4; context.strokeText(point.pointCode, pixel.x + 12, pixel.y - 12); context.fillText(point.pointCode, pixel.x + 12, pixel.y - 12); context.restore(); });
    return canvas.toDataURL("image/png");
  }); }, [onExportReady, points, routeLines]);
  useEffect(() => { onRouteVertexRef.current = onRouteVertex; isDrawingRouteRef.current = isDrawingRoute; }, [isDrawingRoute, onRouteVertex]);
  useEffect(() => { const point = points.find((item) => item.id === selectedPointId); if (point && map.current) map.current.easeTo({ center: [point.longitude, point.latitude], duration: 350 }); }, [points, selectedPointId]);
  useEffect(() => { map.current?.setBearing(bearing); }, [bearing]);
  useEffect(() => { if (focusCenter && map.current) map.current.easeTo({ center: focusCenter, duration: 350 }); }, [focusCenter]);
  useEffect(() => {
    if (!fitToData || !map.current || !points.length) return;
    const coordinates = [...points.map((point) => [point.longitude, point.latitude] as [number, number]), ...routeLines.flatMap((route) => route.coordinates)];
    const west = Math.min(...coordinates.map(([longitude]) => longitude)); const east = Math.max(...coordinates.map(([longitude]) => longitude));
    const south = Math.min(...coordinates.map(([, latitude]) => latitude)); const north = Math.max(...coordinates.map(([, latitude]) => latitude));
    map.current.fitBounds([[west, south], [east, north]], { bearing, duration: 0, maxZoom: 16, padding: 64 });
    fittedZoom.current = map.current.getZoom();
    map.current.setZoom(fittedZoom.current + zoomAdjustment);
  }, [bearing, fitDataKey, fitToData]);
  useEffect(() => { if (fitToData && fittedZoom.current !== null) map.current?.setZoom(fittedZoom.current + zoomAdjustment); }, [fitToData, zoomAdjustment]);

  useEffect(() => {
    if (!container.current || map.current) return;
    map.current = new Map({
      container: container.current,
      center: banqiaoCenter,
      bearing,
      interactive,
      canvasContextAttributes: { preserveDrawingBuffer: true },
      zoom: 13,
      style: nlscBasemapStyle,
    });
    if (interactive) {
      map.current.scrollZoom.enable();
      map.current.dragPan.enable();
      map.current.doubleClickZoom.enable();
      map.current.touchZoomRotate.enable();
    }
    if (showNavigation) map.current.addControl(new NavigationControl(), "top-left");
    map.current.on("error", (event) => {
      if (!hasFallback.current && event.sourceId === "basemap") {
        hasFallback.current = true;
        map.current?.setStyle(fallbackBasemapStyle);
      }
    });
    map.current.on("click", (event) => { if (isDrawingRouteRef.current) onRouteVertexRef.current(event.lngLat.lat, event.lngLat.lng); else onMapClickRef.current(event.lngLat.lat, event.lngLat.lng); });
    map.current.on("mousemove", (event) => onMapPointerMoveRef.current?.(event.lngLat.lat, event.lngLat.lng));
    map.current.on("rotate", () => onBearingChangeRef.current?.(map.current?.getBearing() ?? bearing));
    return () => { map.current?.remove(); map.current = null; };
  }, []);

  useEffect(() => {
    if (!map.current) return;
    const markers = points.map((point) => { const element = document.createElement("button"); let suppressLabelClick = false; element.className = `duty-point-dot ${point.color} ${point.pointType} ${selectedPointId === point.id ? "selected" : ""} ${isDrawingRoute ? "drawing-disabled" : ""} ${showPointLabels ? "show-label" : ""} ${(showPersonnelLabels || personnelLabelPointId === point.id) && personnelLabels[point.id]?.length ? "show-personnel-label" : ""}`; element.title = point.pointName; const label = document.createElement("span"); label.className = "duty-point-label"; label.textContent = point.pointCode; label.title = "拖曳可調整標籤位置（限點位周圍）"; label.addEventListener("pointerdown", (event) => { event.preventDefault(); event.stopPropagation(); const originX = event.clientX; const originY = event.clientY; const startX = Number.parseFloat(element.style.getPropertyValue("--label-offset-x")) || 0; const startY = Number.parseFloat(element.style.getPropertyValue("--label-offset-y")) || 0; const move = (moveEvent: PointerEvent) => { const offsetX = startX + moveEvent.clientX - originX; const offsetY = startY + originY - moveEvent.clientY; const distance = Math.hypot(offsetX, offsetY); const scale = distance > 64 ? 64 / distance : 1; if (Math.abs(moveEvent.clientX - originX) > 2 || Math.abs(moveEvent.clientY - originY) > 2) suppressLabelClick = true; element.style.setProperty("--label-offset-x", `${offsetX * scale}px`); element.style.setProperty("--label-offset-y", `${offsetY * scale}px`); }; const end = () => { window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", end); window.setTimeout(() => { suppressLabelClick = false; }, 0); }; window.addEventListener("pointermove", move); window.addEventListener("pointerup", end); }); element.append(label); const people = personnelLabels[point.id]; if (people?.length) { const personnelLabel = document.createElement("span"); personnelLabel.className = "duty-personnel-label"; personnelLabel.textContent = people.join("、"); element.append(personnelLabel); } element.addEventListener("click", (event) => { event.preventDefault(); event.stopPropagation(); if (suppressLabelClick) return; if (!isDrawingRoute) { onPointSelect(point.id); element.classList.toggle("show-label"); } }); const marker = new Marker({ element, draggable: !isDrawingRoute }).setLngLat([point.longitude, point.latitude]).addTo(map.current!); marker.on("dragend", () => { const position = marker.getLngLat(); onPointMoved(point, position.lat, position.lng); }); return marker; });
    return () => markers.forEach((marker) => marker.remove());
  }, [interactive, isDrawingRoute, onPointMoved, onPointSelect, personnelLabelPointId, personnelLabels, points, selectedPointId, showPersonnelLabels, showPointLabels]);

  useEffect(() => {
    if (!map.current || !pendingCoordinate) return;
    const element = document.createElement("button"); element.className = `duty-point-dot pending ${pendingColor} ${pendingPointType}`; element.title = "取消放置此點位"; element.addEventListener("click", (event) => { event.preventDefault(); event.stopPropagation(); onPendingCancel(); });
    const marker = new Marker({ element }).setLngLat([pendingCoordinate.longitude, pendingCoordinate.latitude]).addTo(map.current);
    return () => { marker.remove(); };
  }, [pendingColor, pendingCoordinate, pendingPointType]);

  useEffect(() => {
    if (!map.current) return;
    const markers = manualVertices.map((vertex, index) => { const element = document.createElement("span"); element.className = `route-draw-vertex ${manualVertexColor}`; element.textContent = String(index + 1); return new Marker({ element }).setLngLat(vertex).addTo(map.current!); });
    return () => markers.forEach((marker) => marker.remove());
  }, [manualVertexColor, manualVertices]);

  useEffect(() => {
    const renderLines = () => {
      if (!map.current?.isStyleLoaded()) return;
      const data = { type: "FeatureCollection" as const, features: routeLines.map((route) => ({ type: "Feature" as const, properties: {}, geometry: { type: "LineString" as const, coordinates: route.coordinates } })) };
      const source = map.current.getSource("duty-routes") as import("maplibre-gl").GeoJSONSource | undefined;
      if (source) source.setData(data); else { map.current.addSource("duty-routes", { type: "geojson", data }); map.current.addLayer({ id: "duty-routes", type: "line", source: "duty-routes", paint: { "line-color": "#1769aa", "line-width": 2.5, "line-opacity": 0.6, "line-dasharray": [2, 2] } }); }
    };
    if (map.current?.isStyleLoaded()) renderLines(); else map.current?.once("load", renderLines);
  }, [routeLines]);

  useEffect(() => {
    if (!container.current || !map.current) return;
    const overlay = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    overlay.classList.add("route-overlay");
    container.current.append(overlay);
    const render = () => {
      const activeMap = map.current;
      if (!activeMap) return;
      overlay.replaceChildren();
      overlay.setAttribute("viewBox", `0 0 ${activeMap.getCanvas().clientWidth} ${activeMap.getCanvas().clientHeight}`);
      const defs = document.createElementNS("http://www.w3.org/2000/svg", "defs");
      overlay.append(defs);
      routeLines.forEach((route) => {
        const color = route.color === "red" ? "#df5050" : route.color === "orange" ? "#ed9a3a" : route.color === "yellow" ? "#f6c453" : route.color === "green" ? "#3faf71" : route.color === "purple" ? "#8966d1" : "#2d9cdb";
        const routeIndex = routeLines.indexOf(route);
        if (route.arrow) { const marker = document.createElementNS("http://www.w3.org/2000/svg", "marker"); marker.setAttribute("id", `route-arrow-${routeIndex}`); marker.setAttribute("markerWidth", "8"); marker.setAttribute("markerHeight", "8"); marker.setAttribute("refX", "7"); marker.setAttribute("refY", "4"); marker.setAttribute("orient", "auto"); const path = document.createElementNS("http://www.w3.org/2000/svg", "path"); path.setAttribute("d", "M 0 0 L 8 4 L 0 8 z"); path.setAttribute("fill", color); marker.append(path); defs.append(marker); }
        const line = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
        line.setAttribute("points", route.coordinates.map((coordinate) => { const pixel = activeMap.project(coordinate); return `${pixel.x},${pixel.y}`; }).join(" "));
        line.setAttribute("fill", "none");
        line.setAttribute("stroke", color);
        line.setAttribute("stroke-width", "2.5");
        line.setAttribute("stroke-linecap", "round");
        line.setAttribute("stroke-linejoin", "round");
        line.setAttribute("opacity", String(route.opacity ?? 1));
        if (route.dashed) line.setAttribute("stroke-dasharray", "10 8");
        if (route.arrow) line.setAttribute("marker-end", `url(#route-arrow-${routeIndex})`);
        overlay.append(line);
      });
    };
    render();
    map.current.on("move", render);
    map.current.on("resize", render);
    return () => { map.current?.off("move", render); map.current?.off("resize", render); overlay.remove(); };
  }, [routeLines]);

  return <div className="map-canvas" ref={container} aria-label="板橋勤務地圖" />;
}
