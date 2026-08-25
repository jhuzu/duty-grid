CREATE TABLE IF NOT EXISTS duty_points (
  id TEXT PRIMARY KEY NOT NULL,
  plan_id TEXT NOT NULL REFERENCES duty_plans(id) ON DELETE CASCADE,
  point_code TEXT NOT NULL,
  point_name TEXT NOT NULL,
  note TEXT,
  latitude REAL NOT NULL,
  longitude REAL NOT NULL,
  visible INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(plan_id, point_code)
);

CREATE INDEX IF NOT EXISTS idx_duty_points_plan_id ON duty_points(plan_id);
