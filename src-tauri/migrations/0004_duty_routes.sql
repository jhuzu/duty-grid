CREATE TABLE IF NOT EXISTS duty_routes (
  id TEXT PRIMARY KEY NOT NULL,
  plan_id TEXT NOT NULL REFERENCES duty_plans(id) ON DELETE CASCADE,
  route_name TEXT NOT NULL,
  color TEXT NOT NULL DEFAULT 'blue',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS duty_route_stops (
  route_id TEXT NOT NULL REFERENCES duty_routes(id) ON DELETE CASCADE,
  point_id TEXT NOT NULL REFERENCES duty_points(id) ON DELETE RESTRICT,
  stop_order INTEGER NOT NULL,
  PRIMARY KEY(route_id, point_id),
  UNIQUE(route_id, stop_order)
);
