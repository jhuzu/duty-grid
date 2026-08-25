import { useEffect, useRef } from "react";
import { Map, NavigationControl, type Map as MapLibreMap } from "maplibre-gl";
import "maplibre-gl/dist/maplibre-gl.css";

const banqiaoCenter: [number, number] = [121.461, 25.012];

export function MapCanvas() {
  const container = useRef<HTMLDivElement>(null);
  const map = useRef<MapLibreMap | null>(null);

  useEffect(() => {
    if (!container.current || map.current) return;
    map.current = new Map({
      container: container.current,
      center: banqiaoCenter,
      zoom: 13,
      style: {
        version: 8,
        sources: {
          openstreetmap: {
            type: "raster",
            tiles: ["https://tile.openstreetmap.org/{z}/{x}/{y}.png"],
            tileSize: 256,
            attribution: "© OpenStreetMap contributors",
          },
        },
        layers: [{ id: "openstreetmap", type: "raster", source: "openstreetmap" }],
      },
    });
    map.current.addControl(new NavigationControl(), "top-left");
    return () => { map.current?.remove(); map.current = null; };
  }, []);

  return <div className="map-canvas" ref={container} aria-label="板橋勤務地圖" />;
}
