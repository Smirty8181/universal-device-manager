use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::model::*;

pub struct AppState {
    pub data_dir: PathBuf,
    pub log_path: PathBuf,
    pub db_path: PathBuf,
    pub db: Option<Mutex<rusqlite::Connection>>,
    pub started_at: u64,
    pub settings: Mutex<AppSettings>,
    pub devices: Mutex<Vec<DeviceView>>,
    pub profiles: Mutex<Vec<Profile>>,
    pub macros: Mutex<Vec<Macro>>,
    pub battery_history: Mutex<Vec<BatterySnapshot>>,
    pub driver_history: Mutex<Vec<DriverHistoryEntry>>,
    pub battery_rgb: Mutex<HashMap<String, BatteryRgbConfig>>,
    pub last_battery_notice: Mutex<HashMap<String, u64>>,
    pub input_map: Mutex<InputMap>,
}

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn set_log_path(path: PathBuf) {
    let _ = LOG_PATH.set(path);
}

pub fn log_path() -> PathBuf {
    LOG_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("udm.log"))
}

pub fn log_line(msg: &str) {
    let Some(path) = LOG_PATH.get() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let stamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{stamp}] {msg}\n");
    use std::io::Write;
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

pub fn read_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<T>(&raw).ok())
        .unwrap_or_default()
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, raw).map_err(|e| e.to_string())?;
    if path.exists() {
        let _ = fs::remove_file(path);
    }
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let log_path = data_dir.join("logs").join("udm.log");
        set_log_path(log_path.clone());
        let settings: AppSettings = read_json(&data_dir.join("settings.json"));
        let profiles: Vec<Profile> = read_json(&data_dir.join("profiles.json"));
        let macros: Vec<Macro> = read_json(&data_dir.join("macros.json"));
        let battery_history: Vec<BatterySnapshot> = read_json(&data_dir.join("battery.json"));
        let driver_history: Vec<DriverHistoryEntry> = read_json(&data_dir.join("drivers.json"));
        let battery_rgb: HashMap<String, BatteryRgbConfig> =
            read_json(&data_dir.join("battery-rgb.json"));
        let input_map: InputMap = read_json(&data_dir.join("input-map.json"));
        let db_path = data_dir.join(crate::db::DB_FILE);
        let db = match crate::db::open(&data_dir) {
            Ok(connection) => Some(Mutex::new(connection)),
            Err(error) => {
                log_line(&format!("device database unavailable: {error}"));
                None
            }
        };
        Self {
            data_dir,
            log_path,
            db_path,
            db,
            started_at: now_ms(),
            settings: Mutex::new(settings),
            devices: Mutex::new(Vec::new()),
            profiles: Mutex::new(profiles),
            macros: Mutex::new(macros),
            battery_history: Mutex::new(battery_history),
            driver_history: Mutex::new(driver_history),
            battery_rgb: Mutex::new(battery_rgb),
            last_battery_notice: Mutex::new(HashMap::new()),
            input_map: Mutex::new(input_map),
        }
    }

    pub fn settings(&self) -> AppSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<(), String> {
        *self.settings.lock().unwrap() = settings.clone();
        write_json(&self.data_dir.join("settings.json"), settings)
    }

    pub fn save_profiles(&self) -> Result<(), String> {
        let list = self.profiles.lock().unwrap().clone();
        write_json(&self.data_dir.join("profiles.json"), &list)
    }

    pub fn save_macros(&self) -> Result<(), String> {
        let list = self.macros.lock().unwrap().clone();
        write_json(&self.data_dir.join("macros.json"), &list)
    }

    pub fn save_battery_history(&self) -> Result<(), String> {
        let list = self.battery_history.lock().unwrap().clone();
        write_json(&self.data_dir.join("battery.json"), &list)
    }

    pub fn save_driver_history(&self) -> Result<(), String> {
        let list = self.driver_history.lock().unwrap().clone();
        write_json(&self.data_dir.join("drivers.json"), &list)
    }

    pub fn save_battery_rgb(&self) -> Result<(), String> {
        let list = self.battery_rgb.lock().unwrap().clone();
        write_json(&self.data_dir.join("battery-rgb.json"), &list)
    }

    pub fn save_input_map(&self) -> Result<(), String> {
        let map = self.input_map.lock().unwrap().clone();
        write_json(&self.data_dir.join("input-map.json"), &map)
    }
}
