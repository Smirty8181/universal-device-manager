CREATE TABLE IF NOT EXISTS telemetry (
    device_id TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_telemetry_updated ON telemetry(updated_at);
