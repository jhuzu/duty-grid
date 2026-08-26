CREATE TABLE IF NOT EXISTS deployment_equipment (
  plan_id TEXT NOT NULL REFERENCES duty_plans(id) ON DELETE CASCADE,
  duty_point_id TEXT NOT NULL REFERENCES duty_points(id) ON DELETE CASCADE,
  selected_items_json TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (plan_id, duty_point_id)
);
