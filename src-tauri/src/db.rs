use std::fs;
use std::path::Path;
use std::time::Instant;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::model::Capability;
use crate::registry::{self, RegistryEntry, RegistrySource};
use crate::store::log_line;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("../migrations/0001_init.sql")),
    (
        "0002_bindings",
        include_str!("../migrations/0002_bindings.sql"),
    ),
    (
        "0003_telemetry",
        include_str!("../migrations/0003_telemetry.sql"),
    ),
];

const REGISTRY_VERSION: &str = "2";

pub const DB_FILE: &str = "device-db.sqlite";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbInfo {
    pub path: String,
    pub schema_version: i64,
    pub migrations: Vec<String>,
    pub device_count: i64,
    pub capability_count: i64,
    pub source_count: i64,
    pub last_updated: Option<String>,
    pub size_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbHistoryEntry {
    pub id: i64,
    pub ts: String,
    pub action: String,
    pub device_id: Option<String>,
    pub details: Option<String>,
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

pub fn open(dir: &Path) -> Result<Connection, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(DB_FILE);
    let mut conn = Connection::open(&path).map_err(|e| e.to_string())?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    migrate(&mut conn)?;
    seed(&mut conn)?;
    sync_registry(&mut conn)?;
    Ok(conn)
}

fn meta_value(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row(
        "SELECT value FROM meta WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn sync_registry(conn: &mut Connection) -> Result<(), String> {
    if meta_value(conn, "registry_version").as_deref() == Some(REGISTRY_VERSION) {
        return Ok(());
    }
    let ts = now();
    let entries = registry::built_in_registry();
    let bindings = registry::built_in_bindings();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for entry in &entries {
        tx.execute("DELETE FROM devices WHERE id = ?1", params![entry.id])
            .map_err(|e| e.to_string())?;
        insert_entry(&tx, entry, &ts)?;
    }
    for binding in &bindings {
        upsert_binding(&tx, binding)?;
    }
    tx.execute(
        "INSERT INTO history (ts, action, details) VALUES (?1, 'registry-sync', ?2)",
        params![
            ts,
            format!(
                "built-in registry {} · {} bindings",
                entries.len(),
                bindings.len()
            )
        ],
    )
    .map_err(|e| e.to_string())?;
    set_meta(&tx, "registry_version", REGISTRY_VERSION)?;
    tx.commit().map_err(|e| e.to_string())?;
    log_line(&format!(
        "db registry synced: {} devices, {} bindings",
        entries.len(),
        bindings.len()
    ));
    Ok(())
}

fn upsert_binding(conn: &Connection, binding: &registry::AdapterBinding) -> Result<(), String> {
    conn.execute(
        "INSERT INTO adapter_bindings (
            id, adapter_id, adapter_label, vid, pid, hardware_id, registry_id,
            priority, verification, source
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
         ON CONFLICT(id) DO UPDATE SET
            adapter_id = excluded.adapter_id,
            adapter_label = excluded.adapter_label,
            vid = excluded.vid,
            pid = excluded.pid,
            hardware_id = excluded.hardware_id,
            registry_id = excluded.registry_id,
            priority = excluded.priority,
            verification = excluded.verification,
            source = excluded.source",
        params![
            binding.id,
            binding.adapter_id,
            binding.adapter_label,
            binding.vid,
            binding.pid,
            binding.hardware_id,
            binding.registry_id,
            binding.priority,
            binding.verification,
            binding.source,
        ],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn load_bindings(conn: &Connection) -> Result<Vec<registry::AdapterBinding>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, adapter_id, adapter_label, vid, pid, hardware_id, registry_id,
                    priority, verification, source
             FROM adapter_bindings ORDER BY priority, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(registry::AdapterBinding {
                id: row.get(0)?,
                adapter_id: row.get(1)?,
                adapter_label: row.get(2)?,
                vid: row.get::<_, i64>(3)? as u16,
                pid: row.get::<_, i64>(4)? as u16,
                hardware_id: row.get(5)?,
                registry_id: row.get(6)?,
                priority: row.get::<_, i64>(7)? as i32,
                verification: row.get(8)?,
                source: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn load_catalog(
    conn: &Connection,
) -> Result<(Vec<RegistryEntry>, Vec<registry::AdapterBinding>), String> {
    Ok((load_registry(conn)?, load_bindings(conn)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportSource {
    label: String,
    url: String,
}

fn default_category() -> String {
    "other".into()
}

fn default_adapter() -> String {
    "generic".into()
}

fn default_adapter_label() -> String {
    "Generic Adapter".into()
}

fn default_verification() -> String {
    "experimental".into()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImportEntry {
    id: String,
    #[serde(default)]
    vendor: String,
    #[serde(default)]
    model: String,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default = "default_adapter")]
    adapter_id: String,
    #[serde(default = "default_adapter_label")]
    adapter_label: String,
    #[serde(default)]
    vid_pids: Vec<String>,
    #[serde(default)]
    hardware_ids: Vec<String>,
    #[serde(default)]
    hardware_revision: Option<String>,
    #[serde(default)]
    firmware: Vec<String>,
    #[serde(default)]
    connections: Vec<String>,
    #[serde(default)]
    protocol_version: Option<String>,
    #[serde(default)]
    protocol_ru: String,
    #[serde(default)]
    protocol_en: String,
    #[serde(default = "default_verification")]
    verification: String,
    #[serde(default)]
    last_checked: Option<String>,
    #[serde(default)]
    official_software: Option<String>,
    #[serde(default)]
    capabilities: Vec<Capability>,
    #[serde(default)]
    sources: Vec<ImportSource>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ImportPayload {
    One(Box<ImportEntry>),
    Many(Vec<ImportEntry>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported: usize,
    pub bindings: usize,
    pub ids: Vec<String>,
}

pub fn import_registry(conn: &mut Connection, raw: &str) -> Result<ImportSummary, String> {
    let payload: ImportPayload =
        serde_json::from_str(raw).map_err(|e| format!("invalid registry JSON: {e}"))?;
    let items = match payload {
        ImportPayload::One(item) => vec![*item],
        ImportPayload::Many(items) => items,
    };
    if items.is_empty() {
        return Err("registry import is empty".into());
    }
    let ts = now();
    let mut ids = Vec::new();
    let mut binding_count = 0usize;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for item in items {
        let id = item.id.trim().to_string();
        if id.is_empty() {
            return Err("every registry entry needs an id".into());
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(format!(
                "registry id '{id}' contains unsupported characters"
            ));
        }
        let adapter_id = if item.adapter_id.trim().is_empty() {
            "generic".to_string()
        } else {
            item.adapter_id.trim().to_string()
        };
        let adapter_label = if item.adapter_label.trim().is_empty() {
            "Generic Adapter".to_string()
        } else {
            item.adapter_label.trim().to_string()
        };
        let capabilities: Vec<Capability> = item
            .capabilities
            .into_iter()
            .filter(|cap| !cap.id.trim().is_empty())
            .map(|mut cap| {
                if cap.status.trim().is_empty() {
                    cap.status = "research".into();
                }
                cap
            })
            .collect();
        let entry = RegistryEntry {
            id: id.clone(),
            vendor: item.vendor.trim().to_string(),
            model: if item.model.trim().is_empty() {
                id.clone()
            } else {
                item.model.trim().to_string()
            },
            category: item.category.trim().to_string(),
            adapter_id: adapter_id.clone(),
            adapter_label: adapter_label.clone(),
            vid_pids: item.vid_pids,
            hardware_ids: item.hardware_ids,
            hardware_revision: item.hardware_revision,
            firmware: item.firmware,
            connections: item.connections,
            protocol_version: item.protocol_version,
            protocol_ru: item.protocol_ru,
            protocol_en: item.protocol_en,
            verification: item.verification,
            last_checked: item
                .last_checked
                .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string()),
            official_software: item.official_software,
            capabilities,
            sources: item
                .sources
                .into_iter()
                .map(|src| RegistrySource {
                    label: src.label,
                    url: src.url,
                })
                .collect(),
        };
        tx.execute("DELETE FROM devices WHERE id = ?1", params![entry.id])
            .map_err(|e| e.to_string())?;
        insert_entry(&tx, &entry, &ts)?;
        for text in &entry.vid_pids {
            let Some((vid, pid)) = registry::parse_vid_pid(text) else {
                continue;
            };
            let binding = registry::AdapterBinding {
                id: format!("{}-{vid:04X}{pid:04X}", entry.id),
                adapter_id: adapter_id.clone(),
                adapter_label: adapter_label.clone(),
                vid,
                pid,
                hardware_id: None,
                registry_id: Some(entry.id.clone()),
                priority: 20,
                verification: entry.verification.clone(),
                source: "import".into(),
            };
            upsert_binding(&tx, &binding)?;
            binding_count += 1;
        }
        ids.push(id);
    }
    tx.execute(
        "INSERT INTO history (ts, action, details) VALUES (?1, 'registry-import', ?2)",
        params![ts, format!("imported {} devices", ids.len())],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    log_line(&format!(
        "registry imported: {} devices, {} bindings",
        ids.len(),
        binding_count
    ));
    Ok(ImportSummary {
        imported: ids.len(),
        bindings: binding_count,
        ids,
    })
}

fn migrate(conn: &mut Connection) -> Result<(), String> {
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if version as usize > MIGRATIONS.len() {
        return Err("database schema is newer than this build".into());
    }
    for (index, (name, sql)) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(sql).map_err(|e| e.to_string())?;
        tx.pragma_update(None, "user_version", (index + 1) as i64)
            .map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO history (ts, action, details) VALUES (?1, 'migration', ?2)",
            params![now(), name],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        log_line(&format!("db migration applied: {name}"));
    }
    Ok(())
}

fn seed(conn: &mut Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM devices", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if count > 0 {
        return Ok(());
    }
    let ts = now();
    let entries = registry::built_in_registry();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for entry in &entries {
        insert_entry(&tx, entry, &ts)?;
    }
    tx.execute(
        "INSERT INTO history (ts, action, details) VALUES (?1, 'seed', ?2)",
        params![ts, format!("built-in registry: {} devices", entries.len())],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)",
        params!["seeded_at", ts],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    log_line(&format!("db seeded with {} devices", entries.len()));
    Ok(())
}

fn vendor_id(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

fn insert_entry(conn: &Connection, entry: &RegistryEntry, ts: &str) -> Result<(), String> {
    let vid = vendor_id(&entry.vendor);
    conn.execute(
        "INSERT OR IGNORE INTO vendors (id, name) VALUES (?1, ?2)",
        params![vid, entry.vendor],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO devices (
            id, vendor_id, vendor, model, category, adapter_id, adapter_label,
            protocol_version, verification, last_checked, official_software,
            protocol_ru, protocol_en, hardware_revision, created_at, updated_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?15)",
        params![
            entry.id,
            vid,
            entry.vendor,
            entry.model,
            entry.category,
            entry.adapter_id,
            entry.adapter_label,
            entry.protocol_version,
            entry.verification,
            entry.last_checked,
            entry.official_software,
            entry.protocol_ru,
            entry.protocol_en,
            entry.hardware_revision,
            ts,
        ],
    )
    .map_err(|e| e.to_string())?;
    for id in &entry.vid_pids {
        conn.execute(
            "INSERT INTO device_vid_pids (device_id, vid_pid) VALUES (?1, ?2)",
            params![entry.id, id],
        )
        .map_err(|e| e.to_string())?;
    }
    for id in &entry.hardware_ids {
        conn.execute(
            "INSERT INTO device_hardware_ids (device_id, hardware_id) VALUES (?1, ?2)",
            params![entry.id, id],
        )
        .map_err(|e| e.to_string())?;
    }
    for connection in &entry.connections {
        conn.execute(
            "INSERT INTO device_connections (device_id, connection) VALUES (?1, ?2)",
            params![entry.id, connection],
        )
        .map_err(|e| e.to_string())?;
    }
    for version in &entry.firmware {
        conn.execute(
            "INSERT INTO device_firmware (device_id, version) VALUES (?1, ?2)",
            params![entry.id, version],
        )
        .map_err(|e| e.to_string())?;
    }
    for cap in &entry.capabilities {
        conn.execute(
            "INSERT INTO capabilities (device_id, key, status, note_ru, note_en)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![entry.id, cap.id, cap.status, cap.note_ru, cap.note_en],
        )
        .map_err(|e| e.to_string())?;
    }
    for src in &entry.sources {
        conn.execute(
            "INSERT INTO sources (device_id, label, url) VALUES (?1, ?2, ?3)",
            params![entry.id, src.label, src.url],
        )
        .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "INSERT INTO protocols (id, device_id, name, transport, version, status, notes_ru, notes_en, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'unknown', ?6, ?7, ?8)",
        params![
            format!("{}.vendor", entry.id),
            entry.id,
            format!("{} vendor interface", entry.model),
            entry.connections.join(", "),
            entry.protocol_version,
            entry.protocol_ru,
            entry.protocol_en,
            ts,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn list_strings(conn: &Connection, sql: &str, device_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![device_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn load_registry(conn: &Connection) -> Result<Vec<RegistryEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, vendor, model, category, adapter_id, adapter_label, protocol_version,
                    verification, last_checked, official_software, protocol_ru, protocol_en,
                    hardware_revision, updated_at
             FROM devices ORDER BY rowid",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                RegistryEntry {
                    id: row.get(0)?,
                    vendor: row.get(1)?,
                    model: row.get(2)?,
                    category: row.get(3)?,
                    adapter_id: row.get(4)?,
                    adapter_label: row.get(5)?,
                    vid_pids: Vec::new(),
                    hardware_ids: Vec::new(),
                    hardware_revision: row.get(12)?,
                    firmware: Vec::new(),
                    connections: Vec::new(),
                    protocol_version: row.get(6)?,
                    protocol_ru: row.get(10)?,
                    protocol_en: row.get(11)?,
                    verification: row.get(7)?,
                    last_checked: row.get(8)?,
                    official_software: row.get(9)?,
                    capabilities: Vec::new(),
                    sources: Vec::new(),
                },
                row.get::<_, String>(13)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for row in rows {
        let (mut entry, _updated) = row.map_err(|e| e.to_string())?;
        entry.vid_pids = list_strings(
            conn,
            "SELECT vid_pid FROM device_vid_pids WHERE device_id = ?1",
            &entry.id,
        )?;
        entry.hardware_ids = list_strings(
            conn,
            "SELECT hardware_id FROM device_hardware_ids WHERE device_id = ?1",
            &entry.id,
        )?;
        entry.firmware = list_strings(
            conn,
            "SELECT version FROM device_firmware WHERE device_id = ?1",
            &entry.id,
        )?;
        entry.connections = list_strings(
            conn,
            "SELECT connection FROM device_connections WHERE device_id = ?1",
            &entry.id,
        )?;
        let mut caps = conn
            .prepare(
                "SELECT key, status, note_ru, note_en FROM capabilities
                 WHERE device_id = ?1 ORDER BY rowid",
            )
            .map_err(|e| e.to_string())?;
        entry.capabilities = caps
            .query_map(params![entry.id], |row| {
                Ok(Capability {
                    id: row.get(0)?,
                    status: row.get(1)?,
                    note_ru: row.get(2)?,
                    note_en: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut src = conn
            .prepare("SELECT label, url FROM sources WHERE device_id = ?1 ORDER BY rowid")
            .map_err(|e| e.to_string())?;
        entry.sources = src
            .query_map(params![entry.id], |row| {
                Ok(RegistrySource {
                    label: row.get(0)?,
                    url: row.get(1)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Registry record whose VID/PID or hardware IDs match a plugged-in device.
fn hex_pair(text: &str) -> Option<String> {
    let upper = text.to_ascii_uppercase();
    let mut runs: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in upper.chars() {
        if ch.is_ascii_hexdigit() {
            current.push(ch);
        } else {
            if current.len() == 4 {
                runs.push(std::mem::take(&mut current));
            }
            current.clear();
        }
    }
    if current.len() == 4 {
        runs.push(current);
    }
    if runs.len() >= 2 {
        Some(format!("{}{}", runs[0], runs[1]))
    } else {
        None
    }
}

pub fn match_registry(conn: &Connection, hardware_ids: &[String]) -> Option<RegistryEntry> {
    let wanted: Vec<String> = hardware_ids
        .iter()
        .map(|id| id.to_ascii_uppercase())
        .filter(|id| id.contains("VID_"))
        .collect();
    let pairs: Vec<String> = wanted.iter().filter_map(|id| hex_pair(id)).collect();
    let entries = load_registry(conn).ok()?;
    entries.into_iter().find(|entry| {
        let by_hwid = entry.hardware_ids.iter().any(|id| {
            let upper = id.to_ascii_uppercase();
            wanted.iter().any(|want| want == &upper)
        });
        let by_pair = entry
            .vid_pids
            .iter()
            .filter_map(|pair| hex_pair(pair))
            .any(|pair| pairs.contains(&pair));
        by_hwid || by_pair
    })
}

pub fn info(conn: &Connection, path: &Path) -> Result<DbInfo, String> {
    let schema_version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let count = |sql: &str| -> Result<i64, String> {
        conn.query_row(sql, [], |row| row.get(0))
            .map_err(|e| e.to_string())
    };
    let last_updated: Option<String> = conn
        .query_row("SELECT MAX(updated_at) FROM devices", [], |row| row.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    let mut size_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    for suffix in ["-wal", "-shm"] {
        let mut side = path.as_os_str().to_os_string();
        side.push(suffix);
        if let Ok(meta) = fs::metadata(&side) {
            size_bytes += meta.len();
        }
    }
    Ok(DbInfo {
        path: path.display().to_string(),
        schema_version,
        migrations: MIGRATIONS
            .iter()
            .map(|(name, _)| name.to_string())
            .collect(),
        device_count: count("SELECT COUNT(*) FROM devices")?,
        capability_count: count("SELECT COUNT(*) FROM capabilities")?,
        source_count: count("SELECT COUNT(*) FROM sources")?,
        last_updated,
        size_bytes,
    })
}

pub fn history(conn: &Connection, limit: usize) -> Result<Vec<DbHistoryEntry>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, ts, action, device_id, details FROM history
             ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit as i64], |row| {
            Ok(DbHistoryEntry {
                id: row.get(0)?,
                ts: row.get(1)?,
                action: row.get(2)?,
                device_id: row.get(3)?,
                details: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn save_telemetry(conn: &Connection, device_id: &str, payload: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO telemetry (device_id, payload, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(device_id) DO UPDATE SET payload = excluded.payload, updated_at = excluded.updated_at",
        params![device_id, payload, now()],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn load_telemetry(conn: &Connection, device_id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT payload FROM telemetry WHERE device_id = ?1",
        params![device_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn record(
    conn: &Connection,
    action: &str,
    device_id: Option<&str>,
    details: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO history (ts, action, device_id, details) VALUES (?1, ?2, ?3, ?4)",
        params![now(), action, device_id, details],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn backup(conn: &Connection, dir: &Path) -> Result<String, String> {
    let folder = dir.join("backups");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let path = folder.join(format!("device-db-{}.sqlite", stamp()));
    let started = Instant::now();
    conn.execute("VACUUM INTO ?1", params![path.display().to_string()])
        .map_err(|e| e.to_string())?;
    record(
        conn,
        "backup",
        None,
        &format!("{} ({} ms)", path.display(), started.elapsed().as_millis()),
    )?;
    log_line(&format!("db backup: {}", path.display()));
    Ok(path.display().to_string())
}
