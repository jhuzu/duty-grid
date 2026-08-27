ALTER TABLE workspace_states ADD COLUMN deployment_route_id TEXT;
ALTER TABLE workspace_states ADD COLUMN map_output_bearing REAL NOT NULL DEFAULT 90;
