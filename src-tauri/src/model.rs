use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    pub id: String,
    pub status: String,
    pub note_ru: String,
    pub note_en: String,
}

impl Capability {
    pub fn new(id: &str, status: &str, ru: &str, en: &str) -> Self {
        Self {
            id: id.into(),
            status: status.into(),
            note_ru: ru.into(),
            note_en: en.into(),
        }
    }
}

fn default_battery_status() -> String {
    "unknown".into()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryInfo {
    pub percent: Option<u8>,
    pub charging: Option<bool>,
    pub source: String,
    #[serde(default = "default_battery_status")]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_ru: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_en: Option<String>,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryRgbRange {
    pub min: u8,
    pub max: u8,
    pub color: String,
    #[serde(default = "default_range_brightness")]
    pub brightness: u8,
    #[serde(default)]
    pub blink: bool,
}

fn default_range_brightness() -> u8 {
    100
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LightingSnapshot {
    pub effect: u8,
    pub brightness: u8,
    pub speed: u8,
    pub direction: u8,
    pub color: String,
}

fn default_battery_rgb_ranges() -> Vec<BatteryRgbRange> {
    vec![
        BatteryRgbRange {
            min: 91,
            max: 100,
            color: "#22c55e".into(),
            brightness: 100,
            blink: false,
        },
        BatteryRgbRange {
            min: 71,
            max: 90,
            color: "#a3e635".into(),
            brightness: 100,
            blink: false,
        },
        BatteryRgbRange {
            min: 51,
            max: 70,
            color: "#facc15".into(),
            brightness: 100,
            blink: false,
        },
        BatteryRgbRange {
            min: 31,
            max: 50,
            color: "#fb923c".into(),
            brightness: 100,
            blink: false,
        },
        BatteryRgbRange {
            min: 11,
            max: 30,
            color: "#ef4444".into(),
            brightness: 100,
            blink: false,
        },
        BatteryRgbRange {
            min: 0,
            max: 10,
            color: "#ef4444".into(),
            brightness: 100,
            blink: true,
        },
    ]
}

fn default_battery_rgb_layout() -> String {
    "solid".into()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BatteryRgbConfig {
    pub enabled: bool,
    #[serde(default = "default_battery_rgb_ranges")]
    pub ranges: Vec<BatteryRgbRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charging_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charging_brightness: Option<u8>,
    #[serde(default = "default_battery_rgb_layout")]
    pub layout: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved: Option<LightingSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_color: Option<String>,
}

impl Default for BatteryRgbConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ranges: default_battery_rgb_ranges(),
            charging_color: None,
            charging_brightness: None,
            layout: default_battery_rgb_layout(),
            saved: None,
            last_percent: None,
            last_color: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdapterModuleView {
    pub id: String,
    pub label: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub note_ru: String,
    pub note_en: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HidInterface {
    pub path: String,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: Option<i32>,
    pub product: Option<String>,
    pub manufacturer: Option<String>,
    pub serial: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriverInfo {
    pub version: Option<String>,
    pub provider: Option<String>,
    pub date: Option<String>,
    pub service: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceView {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub serial: Option<String>,
    pub connection: String,
    pub connection_label: String,
    pub firmware: Option<String>,
    pub battery: Option<BatteryInfo>,
    pub online: bool,
    pub adapter_id: String,
    pub adapter_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    #[serde(default)]
    pub modules: Vec<AdapterModuleView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    pub capabilities: Vec<Capability>,
    pub interfaces: Vec<HidInterface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hardware_ids: Vec<String>,
    pub driver: Option<DriverInfo>,
    pub physical_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bluetooth_address: Option<String>,
    pub detected_at: u64,
    pub updated_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioEndpoint {
    pub id: String,
    pub name: String,
    pub direction: String,
    pub is_default: bool,
    pub state: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub endpoint_id: String,
    pub volume: f32,
    pub muted: bool,
    pub channels: Vec<f32>,
    pub channel_count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub language: String,
    pub accent: String,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub autostart: bool,
    pub auto_scan: bool,
    pub scan_interval_ms: u64,
    pub auto_switch_profiles: bool,
    pub notifications_enabled: bool,
    pub tray_battery: bool,
    pub low_battery_threshold: u8,
    pub store_history: bool,
    pub auto_update_check: bool,
    pub auto_download_drivers: bool,
    pub game_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: "ru".into(),
            accent: "pink".into(),
            start_minimized: false,
            minimize_to_tray: true,
            autostart: false,
            auto_scan: true,
            scan_interval_ms: 3000,
            auto_switch_profiles: true,
            notifications_enabled: true,
            tray_battery: true,
            low_battery_threshold: 15,
            store_history: true,
            auto_update_check: false,
            auto_download_drivers: true,
            game_mode: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MacroEvent {
    pub t: u64,
    pub kind: String,
    pub label: String,
    pub down: bool,
    pub vk: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub events: Vec<MacroEvent>,
    pub repeat: u32,
    #[serde(rename = "loop")]
    pub loop_enabled: bool,
    pub hotkey: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_device_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_device_pattern: Option<String>,
    pub volume: u8,
    pub muted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channels: Option<Vec<f32>>,
    pub mic_gain: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RgbProfile {
    pub enabled: bool,
    pub effect: String,
    pub color: String,
    pub secondary: String,
    pub brightness: u8,
    pub speed: u8,
    pub direction: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MouseProfile {
    pub dpi: u32,
    pub polling: u32,
    pub lod: f32,
    pub debounce: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardProfile {
    pub layout: String,
    pub game_mode: bool,
    pub battery_light: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub apps: Vec<String>,
    pub audio: AudioProfile,
    pub rgb: RgbProfile,
    pub mouse: MouseProfile,
    pub keyboard: KeyboardProfile,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriverEntry {
    pub instance_id: String,
    pub name: String,
    #[serde(rename = "class")]
    pub class_name: String,
    pub problem_code: u32,
    pub problem_label: Option<String>,
    pub driver_version: Option<String>,
    pub driver_provider: Option<String>,
    pub driver_date: Option<String>,
    pub service: Option<String>,
    pub driver_inf: Option<String>,
    pub hardware_ids: Vec<String>,
    pub signature: String,
    pub usb_vid: Option<u16>,
    pub usb_pid: Option<u16>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriverCandidate {
    pub inf_path: String,
    pub published_name: Option<String>,
    pub description: String,
    pub provider: String,
    pub version: String,
    pub date: Option<String>,
    pub class_name: String,
    pub signature: String,
    pub exact_hwid_match: bool,
    pub newer_than_installed: bool,
    pub is_installed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriverResolution {
    pub instance_id: String,
    pub device_name: String,
    pub hardware_ids: Vec<String>,
    pub installed_version: Option<String>,
    pub installed_inf: Option<String>,
    pub candidates: Vec<DriverCandidate>,
    pub sources: Vec<RegistryLink>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryLink {
    pub label: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogUpdate {
    pub update_id: String,
    pub title: String,
    pub products: String,
    pub classification: String,
    pub date: String,
    pub version: String,
    pub size_bytes: u64,
    pub size_text: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPackage {
    pub update_id: String,
    pub title: String,
    pub dir: String,
    pub files: Vec<String>,
    pub signature: String,
    pub infs: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutoDownloadItem {
    pub device: String,
    pub title: String,
    pub dir: String,
    pub infs: Vec<String>,
    pub size_bytes: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutoDownloadReport {
    pub enabled: bool,
    pub checked: usize,
    pub downloaded: Vec<AutoDownloadItem>,
    pub skipped: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreDriver {
    pub published_name: String,
    pub original_name: String,
    pub provider: String,
    pub class_name: String,
    pub version: String,
    pub date: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareInfo {
    pub instance_id: String,
    pub device_name: String,
    pub hardware_ids: Vec<String>,
    pub revision: Option<String>,
    pub protocol_version: Option<String>,
    pub driver_version: Option<String>,
    pub driver_date: Option<String>,
    pub signature: String,
    pub known_revisions: Vec<String>,
    pub sources: Vec<RegistryLink>,
    pub flashable: bool,
    pub recovery_documented: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatterySnapshot {
    pub device_id: String,
    pub at: u64,
    pub percent: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriverHistoryEntry {
    pub at: u64,
    pub instance_id: String,
    pub name: String,
    pub result: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub app_version: String,
    pub os_version: String,
    pub admin: bool,
    pub data_dir: String,
    pub log_file: Option<String>,
    pub device_count: usize,
    pub adapter_count: usize,
    pub started_at: u64,
    pub tauri: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HidProbeResult {
    pub data: String,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct KeyRemap {
    pub vk: u32,
    pub target: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ButtonRemap {
    pub button: u32,
    pub action: String,
    pub target: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct InputMap {
    pub enabled: bool,
    pub keys: Vec<KeyRemap>,
    pub buttons: Vec<ButtonRemap>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MouseSystem {
    pub pointer_speed: u32,
    pub acceleration: bool,
    pub wheel_lines: u32,
    pub swap_buttons: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct InterceptEvent {
    pub at: u64,
    pub kind: String,
    pub from: u32,
    pub to: Option<u32>,
    pub action: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct InputStatus {
    pub engine_active: bool,
    pub remapping: bool,
    pub seen_keys: u64,
    pub swallowed_keys: u64,
    pub seen_buttons: u64,
    pub swallowed_buttons: u64,
    pub recent: Vec<InterceptEvent>,
    pub map: InputMap,
    pub system: MouseSystem,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PollingStat {
    pub device: String,
    pub transport: String,
    pub kind: String,
    pub vid: Option<u32>,
    pub pid: Option<u32>,
    pub rate: f64,
    pub reports: u32,
    pub window_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSession {
    pub instance_id: String,
    pub name: String,
    pub process_id: u32,
    pub volume: f32,
    pub muted: bool,
    pub system_sounds: bool,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
