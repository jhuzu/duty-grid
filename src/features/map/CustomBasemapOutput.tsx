import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { MapDutyPoint } from "./MapCanvas";

type RouteLine = { color: string; coordinates: [number, number][]; dashed?: boolean; opacity?: number };
const colors: Record<string, string> = { red: "#df5050", orange: "#ed9a3a", yellow: "#f6c453", green: "#3faf71", blue: "#2d9cdb", purple: "#8966d1" };

export function CustomBasemapOutput({ basemapUrl, onExportReady, points, routeLines }: { basemapUrl: string; onExportReady: (exporter: () => string | null) => void; points: MapDutyPoint[]; routeLines: RouteLine[] }) {
  const [target, setTarget] = useState<Element | null>(null);
  const image = useRef<HTMLImageElement>(null);
  const xy = (point: MapDutyPoint): [number, number] => [point.coordinateX ?? point.longitude, point.coordinateY ?? point.latitude];
  useEffect(() => { setTarget(document.querySelector(".map-output-frame")); }, []);
  useEffect(() => { onExportReady(() => {
    const source = image.current; if (!source?.naturalWidth) return null;
    const canvas = document.createElement("canvas"); canvas.width = source.naturalWidth; canvas.height = source.naturalHeight;
    const context = canvas.getContext("2d"); if (!context) return null; context.drawImage(source, 0, 0);
    const pixel = ([x, y]: [number, number]) => [x / 1000 * canvas.width, y / 1000 * canvas.height] as const;
    routeLines.forEach((route) => { if (route.coordinates.length < 2) return; context.save(); context.strokeStyle = colors[route.color] ?? colors.blue; context.globalAlpha = route.opacity ?? 1; context.lineWidth = Math.max(3, canvas.width / 450); if (route.dashed) context.setLineDash([canvas.width / 80, canvas.width / 110]); context.beginPath(); route.coordinates.map(pixel).forEach(([x, y], index) => index ? context.lineTo(x, y) : context.moveTo(x, y)); context.stroke(); context.restore(); });
    points.forEach((point) => { const [x, y] = pixel(xy(point)); const radius = Math.max(9, canvas.width / 110); context.save(); context.fillStyle = point.pointType === "hollow" ? "#fff" : colors[point.color] ?? colors.blue; context.strokeStyle = "#17202b"; context.lineWidth = 2; context.beginPath(); context.arc(x, y, radius, 0, Math.PI * 2); context.fill(); context.stroke(); context.font = `700 ${Math.max(16, canvas.width / 65)}px sans-serif`; context.fillStyle = "#17202b"; context.fillText(point.pointCode, x + radius + 5, y - radius); context.restore(); });
    return canvas.toDataURL("image/png");
  }); }, [onExportReady, points, routeLines]);
  if (!target) return null;
  return createPortal(<div className="custom-basemap-output"><img alt="自選底圖輸出" ref={image} src={basemapUrl} /><svg aria-hidden="true" preserveAspectRatio="none" viewBox="0 0 1000 1000">{routeLines.map((route, index) => <polyline fill="none" key={index} points={route.coordinates.map(([x, y]) => `${x},${y}`).join(" ")} stroke={colors[route.color] ?? colors.blue} strokeDasharray={route.dashed ? "12 9" : undefined} strokeLinecap="round" strokeLinejoin="round" strokeOpacity={route.opacity ?? 1} strokeWidth="4" />)}</svg>{points.map((point) => { const [x, y] = xy(point); return <span className={`custom-map-output-point ${point.color}`} key={point.id} style={{ left: `${x / 10}%`, top: `${y / 10}%` }}>{point.pointCode}</span>; })}</div>, target);
}
