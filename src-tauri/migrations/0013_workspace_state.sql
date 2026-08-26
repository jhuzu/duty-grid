CREATE TABLE IF NOT EXISTS workspace_states (
    plan_id TEXT PRIMARY KEY REFERENCES duty_plans(id) ON DELETE CASCADE,
    active_nav TEXT NOT NULL DEFAULT '勤務計畫',
    selected_point_id TEXT,
    selected_route_id TEXT,
    deployment_choices_json TEXT NOT NULL DEFAULT '{}',
    map_output_title TEXT NOT NULL DEFAULT '',
    map_output_zoom REAL NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
