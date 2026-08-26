ALTER TABLE personnel ADD COLUMN unit TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS personnel_assignments (
  id TEXT PRIMARY KEY NOT NULL,
  plan_id TEXT NOT NULL REFERENCES duty_plans(id) ON DELETE CASCADE,
  personnel_id TEXT NOT NULL REFERENCES personnel(id) ON DELETE RESTRICT,
  duty_point_id TEXT REFERENCES duty_points(id) ON DELETE SET NULL,
  assigned_unit TEXT NOT NULL,
  assigned_title TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(plan_id, personnel_id)
);

CREATE INDEX IF NOT EXISTS idx_personnel_assignments_plan_point ON personnel_assignments(plan_id, duty_point_id);
