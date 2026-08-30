import { useEffect, useRef, useState } from "react";
import type { MapDutyPoint } from "./MapCanvas";

type RouteLine = { color: string; coordinates: [number, number][]; dashed?: boolean; arrow?: boolean; opacity?: number };
type CanvasSize = { width: number; height: number };
const colors: Record<string, string> = { red: "#df5050", orange: "#ed9a3a", yellow: "#f6c453", green: "#3faf71", blue: "#2d9cdb", purple: "#8966d1" };

export function CustomBasemapCanvas({ basemapUrl, focusCoordinates, interactive = true, isDrawingRoute, manualVertices, onCanvasClick, onExportReady, onPointMoved, onPointSelect, pendingCoordinate, personnelLabels, points, routeLines, selectedPointId, showPersonnelLabels }: { basemapUrl: string; focusCoordinates?: [number, number][]; interactive?: boolean; isDrawingRoute: boolean; manualVertices: [number, number][]; onCanvasClick: (x: number, y: number) => void; onExportReady: (exporter: () => string | null) => void; onPointMoved: (point: MapDutyPoint, x: number, y: number) => void; onPointSelect: (pointId: string) => void; pendingCoordinate: { x: number; y: number; color: string } | null; personnelLabels: Record<string, string[]>; points: MapDutyPoint[]; routeLines: RouteLine[]; selectedPointId: string | null; showPersonnelLabels: boolean }) {
  const image = useRef<HTMLImageElement>(null);
  const viewport = useRef<HTMLDivElement>(null);
  const dragged = useRef(false);
  const [imageSize, setImageSize] = useState<CanvasSize>({ width: 1000, height: 1000 });
  const [viewportSize, setViewportSize] = useState<CanvasSize>({ width: 0, height: 0 });
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [personnelLabelOffsets, setPersonnelLabelOffsets] = useState<Record<string, { x: number; y: number }>>({});
  const focusCoordinatesKey = JSON.stringify(focusCoordinates ?? []);
  const pointXY = (point: MapDutyPoint): [number, number] => [point.coordinateX ?? point.longitude, point.coordinateY ?? point.latitude];
  const baseScale = viewportSize.width && viewportSize.height ? Math.min(viewportSize.width / imageSize.width, viewportSize.height / imageSize.height) : 1;
  const baseOffset = { x: (viewportSize.width - imageSize.width * baseScale) / 2, y: (viewportSize.height - imageSize.height * baseScale) / 2 };
  const totalScale = baseScale * zoom;
  const toPixels = ([x, y]: [number, number]) => [x / 1000 * imageSize.width, y / 1000 * imageSize.height] as const;
  const clientToXY = (clientX: number, clientY: number): [number, number] => {
    const box = viewport.current?.getBoundingClientRect();
    if (!box || !totalScale) return [0, 0];
    const imageX = (clientX - box.left - baseOffset.x - pan.x) / totalScale;
    const imageY = (clientY - box.top - baseOffset.y - pan.y) / totalScale;
    return [Math.max(0, Math.min(1000, imageX / imageSize.width * 1000)), Math.max(0, Math.min(1000, imageY / imageSize.height * 1000))];
  };

  useEffect(() => {
    const element = viewport.current; if (!element) return;
    const observer = new ResizeObserver(([entry]) => setViewportSize({ width: entry.contentRect.width, height: entry.contentRect.height }));
    observer.observe(element); return () => observer.disconnect();
  }, []);
  useEffect(() => { setZoom(1); setPan({ x: 0, y: 0 }); }, [basemapUrl]);
  useEffect(() => {
    if (!focusCoordinates?.length || !viewportSize.width || !viewportSize.height || !baseScale) return;
    const coordinates = focusCoordinates.filter(([x, y]) => Number.isFinite(x) && Number.isFinite(y) && x >= 0 && x <= 1000 && y >= 0 && y <= 1000);
    if (!coordinates.length) return;
    const xs = coordinates.map(([x]) => x / 1000 * imageSize.width); const ys = coordinates.map(([, y]) => y / 1000 * imageSize.height);
    const minX = Math.min(...xs); const maxX = Math.max(...xs); const minY = Math.min(...ys); const maxY = Math.max(...ys);
    const padding = 96;
    const nextZoom = coordinates.length === 1 ? Math.min(4, Math.max(1, 1 / baseScale)) : Math.max(0.5, Math.min(4, Math.min((viewportSize.width - padding * 2) / Math.max(maxX - minX, 1) / baseScale, (viewportSize.height - padding * 2) / Math.max(maxY - minY, 1) / baseScale)));
    const centerX = (minX + maxX) / 2; const centerY = (minY + maxY) / 2;
    setZoom(nextZoom);
    setPan({ x: viewportSize.width / 2 - baseOffset.x - centerX * baseScale * nextZoom, y: viewportSize.height / 2 - baseOffset.y - centerY * baseScale * nextZoom });
  }, [baseOffset.x, baseOffset.y, baseScale, focusCoordinatesKey, imageSize.height, imageSize.width, viewportSize.height, viewportSize.width]);
  useEffect(() => { onExportReady(() => {
    const source = image.current; if (!source?.naturalWidth) return null;
    const canvas = document.createElement("canvas"); canvas.width = source.naturalWidth; canvas.height = source.naturalHeight;
    const context = canvas.getContext("2d"); if (!context) return null;
    context.drawImage(source, 0, 0);
    const toPixel = ([x, y]: [number, number]) => [x / 1000 * canvas.width, y / 1000 * canvas.height] as const;
    routeLines.forEach((route) => { if (route.coordinates.length < 2) return; context.save(); context.strokeStyle = colors[route.color] ?? colors.blue; context.globalAlpha = route.opacity ?? 1; context.lineWidth = Math.max(3, canvas.width / 450); if (route.dashed) context.setLineDash([canvas.width / 80, canvas.width / 110]); context.beginPath(); route.coordinates.map(toPixel).forEach(([x, y], index) => index ? context.lineTo(x, y) : context.moveTo(x, y)); context.stroke(); context.restore(); });
    points.forEach((point) => { const [x, y] = toPixel(pointXY(point)); const radius = Math.max(9, canvas.width / 110); context.save(); context.fillStyle = point.pointType === "hollow" ? "#fff" : colors[point.color] ?? colors.blue; context.strokeStyle = "#1d2a36"; context.lineWidth = Math.max(2, radius / 4); context.beginPath(); context.arc(x, y, radius, 0, Math.PI * 2); context.fill(); context.stroke(); context.font = `700 ${Math.max(16, canvas.width / 65)}px sans-serif`; context.strokeStyle = "#fff"; context.lineWidth = 4; context.strokeText(point.pointCode, x + radius + 5, y - radius); context.fillStyle = "#17202b"; context.fillText(point.pointCode, x + radius + 5, y - radius); context.restore(); });
    return canvas.toDataURL("image/png");
  }); }, [onExportReady, points, routeLines]);
  useEffect(() => {
    const focusRoute = (event: Event) => {
      const coordinates = (event as CustomEvent<[number, number][]>).detail ?? [];
      if (!coordinates.length || coordinates.some(([x, y]) => x < 0 || x > 1000 || y < 0 || y > 1000)) return;
      const centerX = coordinates.reduce((sum, [x]) => sum + x, 0) / coordinates.length / 1000 * imageSize.width;
      const centerY = coordinates.reduce((sum, [, y]) => sum + y, 0) / coordinates.length / 1000 * imageSize.height;
      setPan({ x: viewportSize.width / 2 - baseOffset.x - centerX * totalScale, y: viewportSize.height / 2 - baseOffset.y - centerY * totalScale });
    };
    window.addEventListener("dutygrid:focus-route", focusRoute); return () => window.removeEventListener("dutygrid:focus-route", focusRoute);
  }, [baseOffset.x, baseOffset.y, imageSize.height, imageSize.width, totalScale, viewportSize.height, viewportSize.width]);

  return <div className="custom-basemap-canvas" aria-label="自選底圖工作區" ref={viewport} onClick={(event) => { if (dragged.current) { dragged.current = false; return; } if (event.target === event.currentTarget || event.target === image.current) { const [x, y] = clientToXY(event.clientX, event.clientY); onCanvasClick(x, y); } }} onPointerDown={(event) => {
    if (!interactive || isDrawingRoute || (event.target !== event.currentTarget && event.target !== image.current)) return;
    const target = viewport.current; if (!target) return;
    const start = { x: event.clientX, y: event.clientY, panX: pan.x, panY: pan.y }; dragged.current = false; target.setPointerCapture(event.pointerId);
    const move = (moveEvent: PointerEvent) => { const dx = moveEvent.clientX - start.x; const dy = moveEvent.clientY - start.y; if (Math.hypot(dx, dy) > 3) dragged.current = true; setPan({ x: start.panX + dx, y: start.panY + dy }); };
    const end = () => { target.removeEventListener("pointermove", move); target.removeEventListener("pointerup", end); target.removeEventListener("pointercancel", end); target.removeEventListener("lostpointercapture", end); };
    target.addEventListener("pointermove", move); target.addEventListener("pointerup", end); target.addEventListener("pointercancel", end); target.addEventListener("lostpointercapture", end);
  }} onWheel={(event) => { if (!interactive) return; event.preventDefault(); const nextZoom = Math.max(0.5, Math.min(4, zoom * (event.deltaY < 0 ? 1.12 : 1 / 1.12))); const [x, y] = clientToXY(event.clientX, event.clientY); const [pixelX, pixelY] = toPixels([x, y]); const box = viewport.current?.getBoundingClientRect(); setPan({ x: event.clientX - (box?.left ?? 0) - baseOffset.x - pixelX * baseScale * nextZoom, y: event.clientY - (box?.top ?? 0) - baseOffset.y - pixelY * baseScale * nextZoom }); setZoom(nextZoom); }}>
    <div className="custom-basemap-stage" style={{ height: imageSize.height, transform: `translate(${baseOffset.x + pan.x}px, ${baseOffset.y + pan.y}px) scale(${totalScale})`, width: imageSize.width }}>
      <img alt="自選底圖" draggable={false} ref={image} src={basemapUrl} onLoad={(event) => setImageSize({ width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight })} />
      <svg aria-hidden="true" viewBox={`0 0 ${imageSize.width} ${imageSize.height}`}>{routeLines.map((route, index) => <polyline key={index} points={route.coordinates.map(toPixels).map(([x, y]) => `${x},${y}`).join(" ")} fill="none" stroke={colors[route.color] ?? colors.blue} strokeDasharray={route.dashed ? "12 9" : undefined} strokeLinecap="round" strokeLinejoin="round" strokeOpacity={route.opacity ?? 1} strokeWidth="4" />)}</svg>
      {interactive && points.map((point) => { const [x, y] = toPixels(pointXY(point)); const offset = personnelLabelOffsets[point.id] ?? { x: 0, y: 0 }; const people = personnelLabels[point.id]; return <button aria-label={`${point.pointCode} ${point.pointName}`} className={`custom-xy-point ${point.color} ${selectedPointId === point.id ? "selected" : ""}`} key={point.id} style={{ left: x, top: y }} type="button" onClick={(event) => { event.stopPropagation(); onPointSelect(point.id); }} onPointerDown={(event) => { if (isDrawingRoute) return; event.preventDefault(); event.stopPropagation(); const target = event.currentTarget; target.setPointerCapture(event.pointerId); const move = (moveEvent: PointerEvent) => { const [nextX, nextY] = clientToXY(moveEvent.clientX, moveEvent.clientY); const [nextPixelX, nextPixelY] = toPixels([nextX, nextY]); target.style.left = `${nextPixelX}px`; target.style.top = `${nextPixelY}px`; }; const end = (endEvent: PointerEvent) => { const [nextX, nextY] = clientToXY(endEvent.clientX, endEvent.clientY); onPointMoved(point, nextX, nextY); target.removeEventListener("pointermove", move); target.removeEventListener("pointerup", end); }; target.addEventListener("pointermove", move); target.addEventListener("pointerup", end); }}><span>{point.pointCode}</span>{showPersonnelLabels && people?.length ? <span className="custom-personnel-label" style={{ transform: `translate(calc(-50% + ${offset.x}px), calc(-100% + ${offset.y - 8}px))` }} onPointerDown={(event) => { event.preventDefault(); event.stopPropagation(); const target = event.currentTarget; const start = { x: event.clientX, y: event.clientY, offset }; target.setPointerCapture(event.pointerId); const move = (moveEvent: PointerEvent) => { const nextX = start.offset.x + moveEvent.clientX - start.x; const nextY = start.offset.y + moveEvent.clientY - start.y; const distance = Math.hypot(nextX, nextY); const scale = distance > 64 ? 64 / distance : 1; setPersonnelLabelOffsets((current) => ({ ...current, [point.id]: { x: nextX * scale, y: nextY * scale } })); }; const end = () => { target.removeEventListener("pointermove", move); target.removeEventListener("pointerup", end); target.removeEventListener("pointercancel", end); }; target.addEventListener("pointermove", move); target.addEventListener("pointerup", end); target.addEventListener("pointercancel", end); }}>{people.join("、")}</span> : null}</button>; })}
      {pendingCoordinate && <span aria-label="待儲存點位" className={`custom-xy-point pending ${pendingCoordinate.color}`} style={{ left: toPixels([pendingCoordinate.x, pendingCoordinate.y])[0], top: toPixels([pendingCoordinate.x, pendingCoordinate.y])[1] }} />}
      {manualVertices.map(([x, y], index) => { const [pixelX, pixelY] = toPixels([x, y]); return <span className="custom-route-vertex" key={`${x}-${y}-${index}`} style={{ left: pixelX, top: pixelY }}>{index + 1}</span>; })}
    </div>
    <button className="custom-basemap-reset" type="button" onClick={(event) => { event.stopPropagation(); setZoom(1); setPan({ x: 0, y: 0 }); }}>重設底圖</button>
  </div>;
}
