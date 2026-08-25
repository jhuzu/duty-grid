CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS duty_plans (
  id TEXT PRIMARY KEY NOT NULL,
  plan_name TEXT NOT NULL,
  duty_date TEXT,
  start_time TEXT,
  end_time TEXT,
  description TEXT,
  status TEXT NOT NULL DEFAULT 'DRAFT',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_duty_plans_duty_date ON duty_plans(duty_date);
CREATE INDEX IF NOT EXISTS idx_duty_plans_updated_at ON duty_plans(updated_at DESC);

CREATE TABLE IF NOT EXISTS import_batches (
  id TEXT PRIMARY KEY NOT NULL,
  source_file_name TEXT NOT NULL,
  imported_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  total_rows INTEGER NOT NULL DEFAULT 0,
  accepted_rows INTEGER NOT NULL DEFAULT 0,
  rejected_rows INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS personnel (
  id TEXT PRIMARY KEY NOT NULL,
  personnel_code TEXT NOT NULL UNIQUE,
  radio_code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  title TEXT NOT NULL,
  import_batch_id TEXT REFERENCES import_batches(id) ON DELETE SET NULL,
  raw_row_json TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_personnel_name ON personnel(name);
