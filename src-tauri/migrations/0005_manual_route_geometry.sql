ALTER TABLE duty_routes ADD COLUMN route_type TEXT NOT NULL DEFAULT 'point_sequence';
ALTER TABLE duty_routes ADD COLUMN geometry_json TEXT;
