PRAGMA foreign_keys = OFF;

CREATE TABLE personnel_assignments_rebuilt (
  id TEXT PRIMARY KEY NOT NULL,
  plan_id TEXT NOT NULL REFERENCES duty_plans(id) ON DELETE CASCADE,
  personnel_id TEXT NOT NULL REFERENCES personnel(id) ON DELETE RESTRICT,
  duty_point_id TEXT REFERENCES duty_points(id) ON DELETE SET NULL,
  assigned_unit TEXT NOT NULL,
  assigned_title TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(plan_id, personnel_id, duty_point_id)
);

INSERT INTO personnel_assignments_rebuilt(id, plan_id, personnel_id, duty_point_id, assigned_unit, assigned_title, created_at)
SELECT id, plan_id, personnel_id, duty_point_id, assigned_unit, assigned_title, created_at FROM personnel_assignments;

DROP TABLE personnel_assignments;
ALTER TABLE personnel_assignments_rebuilt RENAME TO personnel_assignments;
CREATE INDEX idx_personnel_assignments_plan_point ON personnel_assignments(plan_id, duty_point_id);

PRAGMA foreign_keys = ON;
