PRAGMA foreign_keys = OFF;
CREATE TABLE duty_points_rebuilt (
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
  color TEXT NOT NULL DEFAULT 'blue'
);
INSERT INTO duty_points_rebuilt(id, plan_id, point_code, point_name, note, latitude, longitude, visible, created_at, updated_at, color)
SELECT id, plan_id, point_code, point_name, note, latitude, longitude, visible, created_at, updated_at, color FROM duty_points;
DROP TABLE duty_points;
ALTER TABLE duty_points_rebuilt RENAME TO duty_points;
CREATE INDEX idx_duty_points_plan_id ON duty_points(plan_id);
PRAGMA foreign_keys = ON;
