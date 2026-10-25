CREATE TABLE IF NOT EXISTS meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS vendors (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
  id TEXT PRIMARY KEY,
  vendor_id TEXT NOT NULL REFERENCES vendors(id),
  vendor TEXT NOT NULL,
  model TEXT NOT NULL,
  category TEXT NOT NULL,
  adapter_id TEXT NOT NULL,
  adapter_label TEXT NOT NULL,
  adapter_version TEXT NOT NULL DEFAULT '1',
  protocol_version TEXT,
  verification TEXT NOT NULL DEFAULT 'unknown',
  last_checked TEXT NOT NULL,
  official_software TEXT,
  protocol_ru TEXT NOT NULL DEFAULT '',
  protocol_en TEXT NOT NULL DEFAULT '',
  hardware_revision TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS device_vid_pids (
  device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  vid_pid TEXT NOT NULL,
  PRIMARY KEY (device_id, vid_pid)
);

CREATE TABLE IF NOT EXISTS device_hardware_ids (
  device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  hardware_id TEXT NOT NULL,
  PRIMARY KEY (device_id, hardware_id)
);

CREATE TABLE IF NOT EXISTS device_connections (
  device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  connection TEXT NOT NULL,
  PRIMARY KEY (device_id, connection)
);

CREATE TABLE IF NOT EXISTS device_firmware (
  device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  version TEXT NOT NULL,
  PRIMARY KEY (device_id, version)
);

CREATE TABLE IF NOT EXISTS capabilities (
  device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  key TEXT NOT NULL,
  status TEXT NOT NULL,
  note_ru TEXT NOT NULL DEFAULT '',
  note_en TEXT NOT NULL DEFAULT '',
  PRIMARY KEY (device_id, key)
);

CREATE TABLE IF NOT EXISTS sources (
  device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  url TEXT NOT NULL,
  PRIMARY KEY (device_id, label, url)
);

CREATE TABLE IF NOT EXISTS protocols (
  id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  transport TEXT NOT NULL,
  version TEXT,
  status TEXT NOT NULL DEFAULT 'unknown',
  report_descriptor TEXT,
  interfaces TEXT,
  notes_ru TEXT NOT NULL DEFAULT '',
  notes_en TEXT NOT NULL DEFAULT '',
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  ts TEXT NOT NULL,
  action TEXT NOT NULL,
  device_id TEXT,
  details TEXT
);

CREATE TABLE IF NOT EXISTS proposals (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  ts TEXT NOT NULL,
  kind TEXT NOT NULL,
  payload TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'pending'
);

CREATE INDEX IF NOT EXISTS idx_capabilities_device ON capabilities(device_id);
CREATE INDEX IF NOT EXISTS idx_history_ts ON history(ts);
