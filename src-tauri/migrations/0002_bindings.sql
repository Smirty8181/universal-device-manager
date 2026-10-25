CREATE TABLE IF NOT EXISTS adapter_bindings (
  id TEXT PRIMARY KEY,
  adapter_id TEXT NOT NULL,
  adapter_label TEXT NOT NULL,
  vid INTEGER NOT NULL,
  pid INTEGER NOT NULL,
  hardware_id TEXT,
  registry_id TEXT,
  priority INTEGER NOT NULL DEFAULT 0,
  verification TEXT NOT NULL DEFAULT 'unknown',
  source TEXT NOT NULL DEFAULT 'import'
);

CREATE INDEX IF NOT EXISTS idx_bindings_vid_pid ON adapter_bindings(vid, pid);
