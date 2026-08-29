ALTER TABLE duty_plans ADD COLUMN plan_mode TEXT NOT NULL DEFAULT 'map';
ALTER TABLE duty_plans ADD COLUMN basemap_path TEXT;
ALTER TABLE duty_plans ADD COLUMN basemap_width INTEGER;
ALTER TABLE duty_plans ADD COLUMN basemap_height INTEGER;
ALTER TABLE duty_points ADD COLUMN coordinate_x REAL;
ALTER TABLE duty_points ADD COLUMN coordinate_y REAL;
