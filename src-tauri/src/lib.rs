mod adapters;
mod ajazz;
mod alumix;
mod audio;
mod autoclicker;
mod battery;
mod battery_rgb;
mod ble;
mod catalog;
mod db;
mod devices;
mod drivers;
mod hid_desc;
mod hid_lab;
mod input;
mod macros;
mod model;
mod registry;
mod resolver;
mod store;
mod system;
mod telemetry;

use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use model::*;
use store::AppState;

static ACTIVE_PROFILE: Mutex<Option<String>> = Mutex::new(None);

fn device_signature(list: &[DeviceView]) -> String {
    let mut parts = Vec::with_capacity(list.len());
    for d in list {
        parts.push(format!(
            "{}|{}|{}|{}|{}|{}",
            d.id,
            d.online,
            d.name,
            d.connection_label,
            d.interfaces.len(),
            d.physical_path.clone().unwrap_or_default()
        ));
    }
    parts.join(";")
}

fn preserve_battery(list: &mut [DeviceView], previous: &[DeviceView]) {
    let now = now_ms();
    for device in list.iter_mut() {
        let Some(old) = previous.iter().find(|p| p.id == device.id) else {
            continue;
        };
        let Some(old_battery) = old.battery.as_ref() else {
            continue;
        };
        if device.battery.as_ref().and_then(|b| b.percent).is_some() {
            continue;
        }
        match old_battery.status.as_str() {
            "real" | "charging" => {
                if now.saturating_sub(old_battery.updated_at) <= 15 * 60 * 1000 {
                    device.battery = old.battery.clone();
                }
            }
            "unavailable" => device.battery = old.battery.clone(),
            _ => {}
        }
    }
}

fn merge_presence(mut fresh: Vec<DeviceView>, previous: &[DeviceView]) -> Vec<DeviceView> {
    let now = now_ms();
    for old in previous {
        if fresh.iter().any(|d| d.id == old.id) {
            continue;
        }
        if old.online {
            let mut gone = old.clone();
            gone.online = false;
            gone.updated_at = now;
            if let Some(battery) = gone.battery.as_mut() {
                battery.charging = None;
                battery.status = "disconnected".into();
                battery.note_ru =
                    Some("Устройство отключено; показано последнее известное значение.".into());
                battery.note_en =
                    Some("Device disconnected; the last known value is shown.".into());
            }
            fresh.push(gone);
        } else if now.saturating_sub(old.updated_at) < 7 * 24 * 3600 * 1000 {
            fresh.push(old.clone());
        }
    }
    fresh
}

fn store_devices(app: &AppHandle) -> Vec<DeviceView> {
    let state = app.state::<AppState>();
    let previous = state.devices.lock().unwrap().clone();
    let mut list = devices::enumerate_devices();
    preserve_battery(&mut list, &previous);
    let mut list = merge_presence(list, &previous);
    {
        let mut stored = state.devices.lock().unwrap();
        preserve_battery(&mut list, &stored);
        *stored = list.clone();
    }
    list
}

fn refresh_catalog(app: &AppHandle) {
    let state = app.state::<AppState>();
    let Some(db) = state.db.as_ref() else {
        return;
    };
    let result = {
        let conn = db.lock().unwrap();
        db::load_catalog(&conn)
    };
    match result {
        Ok((entries, bindings)) => {
            let summary = format!(
                "catalog: {} devices, {} bindings",
                entries.len(),
                bindings.len()
            );
            registry::set_catalog(entries, bindings);
            crate::store::log_line(&summary);
        }
        Err(error) => crate::store::log_line(&format!("catalog load failed: {error}")),
    }
}

#[tauri::command]
fn list_devices(app: AppHandle) -> Vec<DeviceView> {
    let state = app.state::<AppState>();
    let existing = state.devices.lock().unwrap().clone();
    if existing.is_empty() {
        store_devices(&app)
    } else {
        existing
    }
}

#[tauri::command]
fn rescan_devices(app: AppHandle) -> Vec<DeviceView> {
    let list = store_devices(&app);
    crate::store::log_line(&format!("rescan: {} devices", list.len()));
    list
}

fn probe_and_store(app: &AppHandle, device_id: &str) -> Result<Option<BatteryInfo>, String> {
    let state = app.state::<AppState>();
    let device = {
        let list = state.devices.lock().unwrap();
        list.iter().find(|d| d.id == device_id).cloned()
    };
    let Some(device) = device else {
        return Err("device not found".into());
    };
    if !device.online {
        return Err("device is offline".into());
    }
    let battery = devices::probe_battery(&device);
    let source = battery.source.clone();
    let proven = matches!(battery.status.as_str(), "real" | "charging");
    {
        let mut list = state.devices.lock().unwrap();
        if let Some(item) = list.iter_mut().find(|d| d.id == device_id) {
            item.battery = Some(battery.clone());
            item.updated_at = now_ms();
            if let Some(module) = item.modules.iter_mut().find(|m| m.id == "battery") {
                if proven {
                    module.status = "working".into();
                }
                module.provider = Some(source);
            }
        }
    }
    let _ = app.emit(
        "battery:update",
        serde_json::json!({ "deviceId": device_id, "battery": &battery }),
    );
    if let Some(percent) = battery.percent {
        let settings = state.settings();
        if settings.store_history {
            let now = now_ms();
            let mut history = state.battery_history.lock().unwrap();
            let last = history
                .iter()
                .rev()
                .find(|h| h.device_id == device_id)
                .map(|h| (h.percent, h.at));
            let should_push = last
                .map(|(old, at)| old != percent || now.saturating_sub(at) >= 10 * 60 * 1000)
                .unwrap_or(true);
            if should_push {
                history.push(BatterySnapshot {
                    device_id: device_id.to_string(),
                    at: now,
                    percent,
                });
                if history.len() > 500 {
                    let drop_count = history.len() - 500;
                    history.drain(0..drop_count);
                }
                drop(history);
                let _ = state.save_battery_history();
            }
        }
        if settings.notifications_enabled && percent <= settings.low_battery_threshold {
            let last = state
                .last_battery_notice
                .lock()
                .unwrap()
                .get(device_id)
                .copied();
            let now = now_ms();
            if last
                .map(|t| now.saturating_sub(t) > 10 * 60 * 1000)
                .unwrap_or(true)
            {
                state
                    .last_battery_notice
                    .lock()
                    .unwrap()
                    .insert(device_id.to_string(), now);
                use tauri_plugin_notification::NotificationExt;
                let _ = app
                    .notification()
                    .builder()
                    .title("Universal Device Manager")
                    .body(format!(
                        "{}: {}% — зарядите устройство",
                        device.name, percent
                    ))
                    .show();
            }
        }
    }
    battery_rgb::apply_if_enabled(app, device_id, &battery);
    Ok(Some(battery))
}

#[tauri::command]
fn probe_battery(app: AppHandle, device_id: String) -> Result<Option<BatteryInfo>, String> {
    probe_and_store(&app, &device_id)
}

#[tauri::command]
fn audio_list() -> Result<Vec<AudioEndpoint>, String> {
    audio::list_endpoints()
}

#[tauri::command]
fn audio_state(endpoint_id: String) -> Result<AudioState, String> {
    audio::endpoint_state(&endpoint_id)
}

#[tauri::command]
fn audio_set_volume(endpoint_id: String, level: f32) -> Result<(), String> {
    audio::set_volume(&endpoint_id, level)
}

#[tauri::command]
fn audio_set_channel(endpoint_id: String, index: u32, level: f32) -> Result<(), String> {
    audio::set_channel(&endpoint_id, index, level)
}

#[tauri::command]
fn audio_set_mute(endpoint_id: String, muted: bool) -> Result<(), String> {
    audio::set_mute(&endpoint_id, muted)
}

#[tauri::command]
fn audio_sessions(endpoint_id: String) -> Result<Vec<AppSession>, String> {
    audio::sessions(&endpoint_id)
}

#[tauri::command]
fn audio_session_volume(
    endpoint_id: String,
    instance_id: String,
    level: f32,
) -> Result<(), String> {
    audio::set_session_volume(&endpoint_id, &instance_id, level)
}

#[tauri::command]
fn audio_session_mute(endpoint_id: String, instance_id: String, muted: bool) -> Result<(), String> {
    audio::set_session_mute(&endpoint_id, &instance_id, muted)
}

#[tauri::command]
fn open_sound_settings() -> Result<(), String> {
    audio::open_sound_settings()
}

#[tauri::command]
fn eapo_status() -> serde_json::Value {
    let (installed, preset_dir) = audio::eapo_status();
    serde_json::json!({ "installed": installed, "presetDir": preset_dir })
}

#[tauri::command]
fn eapo_save_preset(name: String, lines: Vec<String>) -> Result<String, String> {
    audio::eapo_save_preset(&name, &lines)
}

#[tauri::command]
fn macros_list(app: AppHandle) -> Vec<Macro> {
    app.state::<AppState>().macros.lock().unwrap().clone()
}

#[tauri::command]
fn macro_save(app: AppHandle, macro_data: Macro) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut list = state.macros.lock().unwrap();
        if let Some(existing) = list.iter_mut().find(|m| m.id == macro_data.id) {
            *existing = macro_data;
        } else {
            list.push(macro_data);
        }
    }
    state.save_macros()?;
    macros::reload_hotkeys(&app);
    Ok(())
}

#[tauri::command]
fn macro_delete(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.macros.lock().unwrap().retain(|m| m.id != id);
    state.save_macros()?;
    macros::reload_hotkeys(&app);
    Ok(())
}

#[tauri::command]
fn macro_record_start(app: AppHandle) -> Result<(), String> {
    macros::start_recording()?;
    let _ = app.emit("macro:event", serde_json::json!({ "reset": true }));
    Ok(())
}

#[tauri::command]
fn macro_record_stop() -> Result<Macro, String> {
    macros::stop_recording()
}

#[tauri::command]
fn macro_record_cancel() {
    macros::cancel_recording();
}

#[tauri::command]
fn macro_play(app: AppHandle, id: String) -> Result<(), String> {
    let macro_data = {
        let state = app.state::<AppState>();
        let list = state.macros.lock().unwrap();
        list.iter().find(|m| m.id == id).cloned()
    };
    let Some(macro_data) = macro_data else {
        return Err("macro not found".into());
    };
    macros::play(macro_data)
}

#[tauri::command]
fn macro_stop() {
    macros::stop_playback();
}

#[tauri::command]
fn macro_set_hotkey(app: AppHandle, macro_id: String, hotkey: Option<String>) -> bool {
    match hotkey {
        Some(hotkey) => {
            let ok = macros::set_hotkey(&macro_id, &hotkey);
            if ok {
                let state = app.state::<AppState>();
                let mut list = state.macros.lock().unwrap();
                if let Some(item) = list.iter_mut().find(|m| m.id == macro_id) {
                    item.hotkey = Some(hotkey);
                }
                drop(list);
                let _ = state.save_macros();
            }
            ok
        }
        None => {
            macros::reload_hotkeys(&app);
            true
        }
    }
}

#[tauri::command]
fn profiles_list(app: AppHandle) -> Vec<Profile> {
    app.state::<AppState>().profiles.lock().unwrap().clone()
}

#[tauri::command]
fn profile_save(app: AppHandle, profile: Profile) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let mut list = state.profiles.lock().unwrap();
        if let Some(existing) = list.iter_mut().find(|p| p.id == profile.id) {
            *existing = profile;
        } else {
            list.push(profile);
        }
    }
    state.save_profiles()
}

#[tauri::command]
fn profile_delete(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.profiles.lock().unwrap().retain(|p| p.id != id);
    if ACTIVE_PROFILE.lock().unwrap().as_deref() == Some(id.as_str()) {
        *ACTIVE_PROFILE.lock().unwrap() = None;
    }
    state.save_profiles()
}

fn activate_profile(app: &AppHandle, id: &str) -> Result<Profile, String> {
    let state = app.state::<AppState>();
    let profile = {
        let list = state.profiles.lock().unwrap();
        list.iter().find(|p| p.id == id).cloned()
    };
    let Some(profile) = profile else {
        return Err("profile not found".into());
    };
    let applied = audio::apply_profile_audio(
        profile.audio.volume,
        profile.audio.muted,
        profile.audio.channels.as_deref(),
        profile.audio.render_device_pattern.as_deref(),
        profile.audio.capture_device_pattern.as_deref(),
        profile.audio.mic_gain,
    )?;
    *ACTIVE_PROFILE.lock().unwrap() = Some(profile.id.clone());
    crate::store::log_line(&format!(
        "profile activated: {} ({})",
        profile.name, applied
    ));
    let _ = app.emit(
        "profile:activated",
        serde_json::json!({ "id": profile.id, "audio": applied }),
    );
    Ok(profile)
}

#[tauri::command]
fn profile_activate(app: AppHandle, id: String) -> Result<(), String> {
    activate_profile(&app, &id).map(|_| ())
}

#[tauri::command]
fn settings_get(app: AppHandle) -> AppSettings {
    app.state::<AppState>().settings()
}

#[tauri::command]
fn settings_set(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut settings = settings;
    settings.scan_interval_ms = settings.scan_interval_ms.clamp(1000, 600_000);
    let previous = state.settings();
    state.save_settings(&settings)?;
    if previous.language != settings.language {
        crate::store::log_line(&format!("language changed: {}", settings.language));
    }
    if previous.game_mode != settings.game_mode {
        input::set_game_mode(settings.game_mode);
    }
    let _ = app.emit("settings:changed", &settings);
    Ok(())
}

#[tauri::command]
fn autostart_set(app: AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    manager.is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn drivers_scan() -> Vec<DriverEntry> {
    drivers::scan()
}

#[tauri::command]
fn driver_verify(app: AppHandle, instance_id: String) -> Result<DriverEntry, String> {
    let entry = drivers::verify(&instance_id)?;
    let state = app.state::<AppState>();
    drivers::record_history(&state, &entry)?;
    Ok(entry)
}

#[tauri::command]
fn drivers_history(app: AppHandle) -> Vec<DriverHistoryEntry> {
    app.state::<AppState>()
        .driver_history
        .lock()
        .unwrap()
        .clone()
}

#[tauri::command]
async fn driver_resolve(app: AppHandle, instance_id: String) -> Result<DriverResolution, String> {
    let state = app.state::<AppState>();
    resolver::resolve(&state, &instance_id)
}

#[tauri::command]
async fn driver_install(
    app: AppHandle,
    inf_path: String,
    instance_id: String,
    confirm: bool,
) -> Result<String, String> {
    let text = resolver::install(&inf_path, confirm)?;
    let name = resolver::device_name(&instance_id);
    let state = app.state::<AppState>();
    resolver::record_install(
        &state,
        &instance_id,
        &name,
        &format!("Установка пакета: {name}"),
    )?;
    Ok(text)
}

#[tauri::command]
async fn driver_install_elevated(
    app: AppHandle,
    inf_path: String,
    instance_id: String,
    confirm: bool,
) -> Result<String, String> {
    let text = resolver::install_elevated(&inf_path, confirm)?;
    let name = resolver::device_name(&instance_id);
    let state = app.state::<AppState>();
    resolver::record_install(
        &state,
        &instance_id,
        &name,
        &format!("Запрошено повышение прав: {name}"),
    )?;
    Ok(text)
}

#[tauri::command]
async fn driver_store() -> Result<Vec<StoreDriver>, String> {
    resolver::store_list()
}

#[tauri::command]
async fn firmware_info(app: AppHandle, instance_id: String) -> Result<FirmwareInfo, String> {
    let state = app.state::<AppState>();
    resolver::firmware(&state, &instance_id)
}

#[tauri::command]
async fn catalog_search(hardware_id: String) -> Result<Vec<crate::model::CatalogUpdate>, String> {
    tauri::async_runtime::spawn_blocking(move || catalog::search(&hardware_id))
        .await
        .map_err(|e| format!("catalog search task failed: {e}"))?
}

#[tauri::command]
async fn catalog_download(
    app: AppHandle,
    update_id: String,
    title: String,
) -> Result<crate::model::CatalogPackage, String> {
    let data_dir = app.state::<AppState>().data_dir.clone();
    tauri::async_runtime::spawn_blocking(move || catalog::download(&data_dir, &update_id, &title))
        .await
        .map_err(|e| format!("catalog download task failed: {e}"))?
}

#[tauri::command]
async fn driver_auto_download(app: AppHandle) -> Result<crate::model::AutoDownloadReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        Ok(catalog::auto_download(state.inner()))
    })
    .await
    .map_err(|e| format!("auto-download task failed: {e}"))?
}

#[tauri::command]
fn system_info(app: AppHandle) -> SystemInfo {
    let state = app.state::<AppState>();
    system::info(&state, &app.package_info().version.to_string())
}

#[tauri::command]
fn log_tail(lines: usize) -> String {
    system::log_tail(lines)
}

#[tauri::command]
fn device_registry(app: AppHandle) -> Vec<registry::RegistryEntry> {
    let state = app.state::<AppState>();
    let Some(db) = state.db.as_ref() else {
        return registry::built_in_registry();
    };
    let entries = match db::load_registry(&db.lock().unwrap()) {
        Ok(entries) if !entries.is_empty() => entries,
        Ok(_) => registry::built_in_registry(),
        Err(error) => {
            crate::store::log_line(&format!("registry read failed: {error}"));
            registry::built_in_registry()
        }
    };
    entries
}

#[tauri::command]
fn db_info(app: AppHandle) -> Result<db::DbInfo, String> {
    let state = app.state::<AppState>();
    let db = state.db.as_ref().ok_or("device database unavailable")?;
    let result = db::info(&db.lock().unwrap(), &state.db_path);
    result
}

#[tauri::command]
fn db_history(app: AppHandle, limit: Option<usize>) -> Result<Vec<db::DbHistoryEntry>, String> {
    let state = app.state::<AppState>();
    let db = state.db.as_ref().ok_or("device database unavailable")?;
    let result = db::history(&db.lock().unwrap(), limit.unwrap_or(20).clamp(1, 200));
    result
}

#[tauri::command]
fn db_backup(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    let db = state.db.as_ref().ok_or("device database unavailable")?;
    let result = db::backup(&db.lock().unwrap(), &state.data_dir);
    result
}

#[tauri::command]
fn registry_bindings(app: AppHandle) -> Result<Vec<registry::AdapterBinding>, String> {
    let state = app.state::<AppState>();
    let db = state.db.as_ref().ok_or("device database unavailable")?;
    let conn = db.lock().unwrap();
    db::load_bindings(&conn)
}

#[tauri::command]
fn registry_import(app: AppHandle, json: String) -> Result<db::ImportSummary, String> {
    let summary = {
        let state = app.state::<AppState>();
        let db = state.db.as_ref().ok_or("device database unavailable")?;
        let mut conn = db.lock().unwrap();
        db::import_registry(&mut conn, &json)?
    };
    refresh_catalog(&app);
    let list = store_devices(&app);
    let _ = app.emit("devices:update", &list);
    let _ = app.emit(
        "registry:changed",
        serde_json::json!({ "ids": &summary.ids }),
    );
    crate::store::log_line(&format!(
        "registry import: {} entries, {} bindings",
        summary.imported, summary.bindings
    ));
    Ok(summary)
}

#[tauri::command]
fn ble_inspect(app: AppHandle, device_id: String) -> Result<Vec<ble::GattServiceInfo>, String> {
    let device = find_device(&app, &device_id)?;
    let address_text = device
        .bluetooth_address
        .as_deref()
        .ok_or("device has no Bluetooth address")?;
    let address =
        ble::address_from_instance(address_text).ok_or("Bluetooth address could not be parsed")?;
    ble::inspect(address)
}

#[tauri::command]
fn diagnostics_export(app: AppHandle, device_id: String) -> Result<String, String> {
    let device = find_device(&app, &device_id)?;
    let state = app.state::<AppState>();
    let detection = battery::detect(&device.interfaces, device.bluetooth_address.as_deref());
    let mut interfaces = Vec::new();
    for interface in &device.interfaces {
        match hid_desc::caps_for_path(&interface.path) {
            Ok(caps) => {
                let battery_field = hid_desc::battery_field(&caps).map(|field| {
                    format!(
                        "{} report {} · {} bit · logical 0..{}",
                        field.kind, field.report_id, field.bit_size, field.logical_max
                    )
                });
                interfaces.push(serde_json::json!({
                    "path": interface.path,
                    "usagePage": interface.usage_page,
                    "usage": interface.usage,
                    "interfaceNumber": interface.interface_number,
                    "product": interface.product,
                    "manufacturer": interface.manufacturer,
                    "serial": interface.serial,
                    "descriptor": hid_desc::caps_text(&caps),
                    "freeform": hid_desc::usage_summary(&caps.fields),
                    "reportLengths": {
                        "input": caps.input_len,
                        "output": caps.output_len,
                        "feature": caps.feature_len,
                    },
                    "batteryField": battery_field,
                }));
            }
            Err(error) => interfaces.push(serde_json::json!({
                "path": interface.path,
                "usagePage": interface.usage_page,
                "usage": interface.usage,
                "interfaceNumber": interface.interface_number,
                "product": interface.product,
                "manufacturer": interface.manufacturer,
                "serial": interface.serial,
                "error": error,
            })),
        }
    }
    let gatt = match device
        .bluetooth_address
        .as_deref()
        .and_then(ble::address_from_instance)
    {
        Some(address) => match ble::inspect(address) {
            Ok(services) => {
                serde_json::json!({ "address": format!("{address:012X}"), "services": services })
            }
            Err(error) => {
                serde_json::json!({ "address": format!("{address:012X}"), "error": error })
            }
        },
        None => serde_json::json!(null),
    };
    let report = serde_json::json!({
        "generatedAt": now_ms(),
        "appVersion": app.package_info().version.to_string(),
        "os": std::env::consts::OS,
        "device": &device,
        "batteryDetection": {
            "hidBatteryStrength": detection.hid.as_ref().map(|(path, field)| serde_json::json!({
                "path": path,
                "field": field,
            })),
            "bleAddress": detection.ble,
            "result": &device.battery,
        },
        "interfaces": interfaces,
        "gatt": gatt,
        "logTail": system::log_tail(120),
    });
    let reports_dir = state.data_dir.join("reports");
    std::fs::create_dir_all(&reports_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = reports_dir.join(format!(
        "diagnostics-{}-{stamp}.json",
        device.id.replace(|c: char| !c.is_ascii_alphanumeric(), "-")
    ));
    let raw = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?;
    std::fs::write(&path, raw).map_err(|e| e.to_string())?;
    let path_text = path.to_string_lossy().to_string();
    crate::store::log_line(&format!("diagnostics report saved: {path_text}"));
    Ok(path_text)
}

#[tauri::command]
fn clipboard_copy(text: String) -> Result<(), String> {
    system::copy_text(&text)
}

#[tauri::command]
fn hid_read_feature(
    app: AppHandle,
    device_id: String,
    path: String,
    report_id: u8,
    length: usize,
) -> Result<HidProbeResult, String> {
    let device = find_device(&app, &device_id)?;
    hid_lab::read_feature(&device, &path, report_id, length)
}

#[tauri::command]
fn hid_write_feature(
    app: AppHandle,
    device_id: String,
    path: String,
    hex: String,
) -> Result<HidProbeResult, String> {
    let device = find_device(&app, &device_id)?;
    hid_lab::write_feature(&device, &path, &hex)
}

#[tauri::command]
fn hid_descriptor(app: AppHandle, device_id: String) -> Result<String, String> {
    let device = find_device(&app, &device_id)?;
    hid_lab::descriptor(&device)
}

#[tauri::command]
fn mouse_status(app: AppHandle, device_id: String) -> Result<ajazz::MouseStatus, String> {
    let device = find_device(&app, &device_id)?;
    ajazz::read_status(&device)
}

#[tauri::command]
fn mouse_set_dpi(
    app: AppHandle,
    device_id: String,
    stage: usize,
    dpi: u32,
) -> Result<ajazz::MouseCommandResult, String> {
    let device = find_device(&app, &device_id)?;
    ajazz::set_dpi(&device, stage, dpi)
}

#[tauri::command]
fn mouse_set_polling(
    app: AppHandle,
    device_id: String,
    hz: u32,
) -> Result<ajazz::MouseCommandResult, String> {
    let device = find_device(&app, &device_id)?;
    ajazz::set_polling(&device, hz)
}

#[tauri::command]
fn keyboard_status(app: AppHandle, device_id: String) -> Result<alumix::KeyboardStatus, String> {
    let device = find_device(&app, &device_id)?;
    alumix::read_status(&device)
}

#[tauri::command]
fn keyboard_set_lighting(
    app: AppHandle,
    device_id: String,
    effect: u8,
    brightness: u8,
    speed: u8,
    direction: u8,
    color: String,
) -> Result<alumix::LightingCommandResult, String> {
    let device = find_device(&app, &device_id)?;
    alumix::set_lighting(&device, effect, brightness, speed, direction, &color)
}

#[tauri::command]
fn battery_rgb_get(app: AppHandle, device_id: String) -> battery_rgb::BatteryRgbView {
    battery_rgb::view(&app, &device_id)
}

#[tauri::command]
fn battery_rgb_save(
    app: AppHandle,
    device_id: String,
    config: BatteryRgbConfig,
) -> Result<battery_rgb::BatteryRgbView, String> {
    battery_rgb::save(&app, &device_id, config)
}

#[tauri::command]
fn battery_rgb_enable(
    app: AppHandle,
    device_id: String,
    enabled: bool,
) -> Result<battery_rgb::BatteryRgbResult, String> {
    battery_rgb::set_enabled(&app, &device_id, enabled)
}

fn find_device(app: &AppHandle, id: &str) -> Result<DeviceView, String> {
    let state = app.state::<AppState>();
    let list = state.devices.lock().unwrap();
    list.iter()
        .find(|d| d.id == id)
        .cloned()
        .ok_or_else(|| "device not found".into())
}

#[tauri::command]
fn battery_history(app: AppHandle) -> Vec<BatterySnapshot> {
    app.state::<AppState>()
        .battery_history
        .lock()
        .unwrap()
        .clone()
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    autoclicker::stop();
    app.exit(0);
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

const TRAY_LABEL: &str = "tray";
const TRAY_WIDTH: f64 = 372.0;
const TRAY_HEIGHT: f64 = 568.0;

fn ensure_tray_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(TRAY_LABEL) {
        return Ok(window);
    }
    let window = WebviewWindowBuilder::new(app, TRAY_LABEL, WebviewUrl::App("index.html".into()))
        .title("UDM")
        .inner_size(TRAY_WIDTH, TRAY_HEIGHT)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        // Hash is set before page scripts run so the router mounts the compact view only.
        .initialization_script("window.location.hash = '/tray';")
        .build()?;
    let handle = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Focused(false) => {
            let _ = handle.hide();
        }
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = handle.hide();
        }
        _ => {}
    });
    Ok(window)
}

fn position_tray_window(
    app: &AppHandle,
    window: &WebviewWindow,
    at: Option<PhysicalPosition<f64>>,
) {
    let monitor = at
        .and_then(|point| app.monitor_from_point(point.x, point.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        let _ = window.center();
        return;
    };
    let work = monitor.work_area();
    let size = window.outer_size().unwrap_or(tauri::PhysicalSize::new(
        TRAY_WIDTH as u32,
        TRAY_HEIGHT as u32,
    ));
    let anchor = at.unwrap_or_else(|| {
        PhysicalPosition::new(
            (work.position.x + work.size.width as i32 - 24) as f64,
            (work.position.y + work.size.height as i32 - 24) as f64,
        )
    });
    let left = work.position.x + 8;
    let right = work.position.x + work.size.width as i32 - size.width as i32 - 8;
    let top = work.position.y + 8;
    let bottom = work.position.y + work.size.height as i32 - size.height as i32 - 8;
    let x = ((anchor.x as i32) - size.width as i32 / 2).clamp(left, right.max(left));
    let above = (anchor.y as i32) > work.position.y + work.size.height as i32 / 2;
    let y = if above {
        (anchor.y as i32 - size.height as i32 - 14).clamp(top, bottom.max(top))
    } else {
        (anchor.y as i32 + 14).clamp(top, bottom.max(top))
    };
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

fn toggle_tray_window(app: &AppHandle, at: Option<PhysicalPosition<f64>>) {
    let Ok(window) = ensure_tray_window(app) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }
    position_tray_window(app, &window, at);
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit_to(TRAY_LABEL, "tray:refresh", ());
}

fn update_tray_tooltip(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let devices = state.devices.lock().unwrap().clone();
    let online = devices.iter().filter(|d| d.online).count();
    let mut lowest: Option<(&str, u8)> = None;
    if settings.tray_battery {
        for device in devices.iter().filter(|d| d.online) {
            let Some(battery) = &device.battery else {
                continue;
            };
            let Some(percent) = battery.percent else {
                continue;
            };
            if !matches!(battery.status.as_str(), "real" | "charging" | "estimated") {
                continue;
            }
            if lowest.map(|(_, best)| percent < best).unwrap_or(true) {
                lowest = Some((device.name.as_str(), percent));
            }
        }
    }
    let online_label = if settings.language == "en" {
        "online"
    } else {
        "онлайн"
    };
    let text = match lowest {
        Some((name, percent)) => {
            format!("Universal Device Manager · {online} {online_label} · {name} {percent}%")
        }
        None => format!("Universal Device Manager · {online} {online_label}"),
    };
    if let Some(tray) = app.tray_by_id("udm-tray") {
        let _ = tray.set_tooltip(Some(&text));
    }
}

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let mut builder = TrayIconBuilder::with_id("udm-tray")
        .tooltip("Universal Device Manager")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                position,
                ..
            } => toggle_tray_window(tray.app_handle(), Some(position)),
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => show_main(tray.app_handle()),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    let _ = ensure_tray_window(app.handle());
    Ok(())
}

#[tauri::command]
fn tray_hide(app: AppHandle) {
    if let Some(window) = app.get_webview_window(TRAY_LABEL) {
        let _ = window.hide();
    }
}

#[tauri::command]
fn tray_toggle(app: AppHandle) {
    toggle_tray_window(&app, None);
}

#[tauri::command]
fn tray_open(app: AppHandle, route: Option<String>) {
    show_main(&app);
    if let Some(route) = route {
        let _ = app.emit_to("main", "udm:navigate", route);
    }
    if let Some(window) = app.get_webview_window(TRAY_LABEL) {
        let _ = window.hide();
    }
}

fn setup_window_behavior(app: &tauri::App) {
    if let Some(window) = app.get_webview_window("main") {
        let handle = app.handle().clone();
        let win = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let minimize = handle.state::<AppState>().settings().minimize_to_tray;
                if minimize {
                    api.prevent_close();
                    let _ = win.hide();
                }
            }
        });
        let minimized = std::env::args().any(|a| a == "--minimized")
            || app.state::<AppState>().settings().start_minimized;
        if minimized {
            let _ = window.hide();
        }
    }
}

fn foreground_process() -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        result.ok()?;
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        path.rsplit('\\')
            .next()
            .map(|name| name.to_ascii_lowercase())
    }
}

fn needs_battery_probe(device: &DeviceView) -> bool {
    if matches!(device.adapter_id.as_str(), "redsquare" | "ajazz") {
        return true;
    }
    if device.bluetooth_address.is_some() {
        return true;
    }
    device
        .modules
        .iter()
        .any(|module| module.id == "battery" && module.provider.is_some())
}

fn spawn_monitor(app: AppHandle) {
    thread::spawn(move || {
        let mut last_signature = String::new();
        let mut last_app_check = std::time::Instant::now();
        let mut probed: HashMap<String, Instant> = HashMap::new();
        loop {
            let settings = app.state::<AppState>().settings();
            if settings.auto_scan {
                let mut fresh = devices::enumerate_devices();
                let previous = app.state::<AppState>().devices.lock().unwrap().clone();
                preserve_battery(&mut fresh, &previous);
                let mut list = merge_presence(fresh, &previous);
                let signature = device_signature(&list);
                if signature != last_signature {
                    last_signature = signature;
                    let state = app.state::<AppState>();
                    let mut stored = state.devices.lock().unwrap();
                    preserve_battery(&mut list, &stored);
                    *stored = list.clone();
                    drop(stored);
                    let _ = app.emit("devices:update", &list);
                }
            }
            {
                let devices = app.state::<AppState>().devices.lock().unwrap().clone();
                let online: Vec<String> = devices
                    .iter()
                    .filter(|d| d.online)
                    .map(|d| d.id.clone())
                    .collect();
                probed.retain(|id, _| online.contains(id));
                for device in devices
                    .iter()
                    .filter(|d| d.online && needs_battery_probe(d))
                {
                    let due = probed
                        .get(&device.id)
                        .map(|at| at.elapsed() >= Duration::from_secs(60))
                        .unwrap_or(true);
                    if !due {
                        continue;
                    }
                    probed.insert(device.id.clone(), Instant::now());
                    let handle = app.clone();
                    let id = device.id.clone();
                    thread::spawn(move || {
                        if let Err(error) = probe_and_store(&handle, &id) {
                            crate::store::log_line(&format!(
                                "auto battery probe failed for {id}: {error}"
                            ));
                        }
                    });
                }
            }
            if last_app_check.elapsed() >= Duration::from_millis(2000) {
                last_app_check = std::time::Instant::now();
                if settings.auto_switch_profiles {
                    if let Some(exe) = foreground_process() {
                        let profile = {
                            let state = app.state::<AppState>();
                            let list = state.profiles.lock().unwrap();
                            list.iter()
                                .find(|p| {
                                    p.apps.iter().any(|app_name| {
                                        !app_name.trim().is_empty()
                                            && (exe == app_name.to_ascii_lowercase()
                                                || exe.contains(&app_name.to_ascii_lowercase()))
                                    })
                                })
                                .map(|p| p.id.clone())
                        };
                        if let Some(id) = profile {
                            let already =
                                ACTIVE_PROFILE.lock().unwrap().as_deref() == Some(id.as_str());
                            if !already {
                                if let Err(error) = activate_profile(&app, &id) {
                                    crate::store::log_line(&format!(
                                        "profile auto-switch failed for {}: {}",
                                        id, error
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            let interval = app
                .state::<AppState>()
                .settings()
                .scan_interval_ms
                .clamp(1500, 60_000);
            update_tray_tooltip(&app);
            thread::sleep(Duration::from_millis(interval));
        }
    });
}

fn input_snapshot(app: &AppHandle) -> InputStatus {
    let state = app.state::<AppState>();
    let map = state.input_map.lock().unwrap().clone();
    let (seen_keys, swallowed_keys, seen_buttons, swallowed_buttons) = input::counters();
    InputStatus {
        engine_active: input::active(),
        remapping: input::remapping(),
        seen_keys,
        swallowed_keys,
        seen_buttons,
        swallowed_buttons,
        recent: input::history(),
        map,
        system: input::system_get(),
    }
}

fn store_input_map(app: &AppHandle, map: InputMap) -> Result<InputStatus, String> {
    let state = app.state::<AppState>();
    *state.input_map.lock().unwrap() = map.clone();
    state.save_input_map()?;
    input::apply(&map);
    Ok(input_snapshot(app))
}

#[tauri::command]
fn input_status(app: AppHandle) -> InputStatus {
    input_snapshot(&app)
}

#[tauri::command]
fn input_map_set(app: AppHandle, map: InputMap) -> Result<InputStatus, String> {
    let clean = input::sanitize(&map);
    crate::store::log_line(&format!(
        "input map: {} key rules, {} button rules, enabled={}",
        clean.keys.len(),
        clean.buttons.len(),
        clean.enabled
    ));
    let result = store_input_map(&app, clean);
    if result.is_ok() {
        let _ = app.emit("input:changed", ());
    }
    result
}

#[tauri::command]
fn input_enabled_set(app: AppHandle, enabled: bool) -> Result<InputStatus, String> {
    let map = {
        let state = app.state::<AppState>();
        let mut map = state.input_map.lock().unwrap().clone();
        map.enabled = enabled;
        map
    };
    let result = store_input_map(&app, map);
    if result.is_ok() {
        crate::store::log_line(&format!("input remapping enabled={enabled}"));
        let _ = app.emit("input:changed", ());
    }
    result
}

#[tauri::command]
fn mouse_system_get(app: AppHandle) -> MouseSystem {
    let _ = app;
    input::system_get()
}

#[tauri::command]
fn mouse_system_set(app: AppHandle, value: MouseSystem) -> MouseSystem {
    let applied = input::system_set(&value);
    crate::store::log_line(&format!(
        "mouse system settings: speed={} accel={} wheel={} swap={}",
        applied.pointer_speed, applied.acceleration, applied.wheel_lines, applied.swap_buttons
    ));
    let _ = app.emit("input:changed", ());
    applied
}

#[tauri::command]
fn key_catalog() -> Vec<(u32, String)> {
    input::key_names()
}

#[tauri::command]
fn polling_stats(window_ms: Option<u64>) -> Vec<PollingStat> {
    input::stats(window_ms.unwrap_or(1000))
}

#[tauri::command]
fn polling_reset() {
    input::reset_stats();
}

#[tauri::command]
async fn telemetry_scan(
    app: AppHandle,
    device_id: String,
) -> Result<telemetry::TelemetryReport, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let device = find_device(&handle, &device_id)?;
        if !device.online {
            return Err("device is offline".into());
        }
        let mut report = telemetry::scan(&device);
        report.source = "live".into();
        let state = handle.state::<AppState>();
        if let Some(db) = state.db.as_ref() {
            match serde_json::to_string(&report) {
                Ok(payload) => {
                    if let Err(error) =
                        db::save_telemetry(&db.lock().unwrap(), &device_id, &payload)
                    {
                        crate::store::log_line(&format!("telemetry cache save failed: {error}"));
                    }
                }
                Err(error) => {
                    crate::store::log_line(&format!("telemetry cache encode failed: {error}"))
                }
            }
        }
        Ok(report)
    })
    .await
    .map_err(|e| format!("telemetry task failed: {e}"))?
}

#[tauri::command]
fn telemetry_cached(
    app: AppHandle,
    device_id: String,
) -> Result<Option<telemetry::TelemetryReport>, String> {
    let state = app.state::<AppState>();
    let Some(db) = state.db.as_ref() else {
        return Ok(None);
    };
    let raw = {
        let conn = db.lock().unwrap();
        db::load_telemetry(&conn, &device_id)?
    };
    let Some(raw) = raw else {
        return Ok(None);
    };
    match serde_json::from_str::<telemetry::TelemetryReport>(&raw) {
        Ok(mut report) => {
            report.source = "cache".into();
            Ok(Some(report))
        }
        Err(error) => {
            crate::store::log_line(&format!("telemetry cache parse failed: {error}"));
            Ok(None)
        }
    }
}

#[tauri::command]
async fn protocol_feature_sweep(
    app: AppHandle,
    device_id: String,
    path: String,
) -> Result<Vec<telemetry::FeatureProbe>, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let device = find_device(&handle, &device_id)?;
        telemetry::feature_sweep(&device, &path)
    })
    .await
    .map_err(|e| format!("feature sweep task failed: {e}"))?
}

#[tauri::command]
async fn protocol_capture(
    app: AppHandle,
    device_id: String,
    path: String,
    duration_ms: Option<u64>,
) -> Result<telemetry::CaptureResult, String> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let device = find_device(&handle, &device_id)?;
        telemetry::capture(&device, &path, duration_ms.unwrap_or(1200))
    })
    .await
    .map_err(|e| format!("capture task failed: {e}"))?
}

fn refresh_live_sources(app: &AppHandle) {
    let devices = app.state::<AppState>().devices.lock().unwrap().clone();
    let mut entries = Vec::new();
    for device in devices.iter() {
        for iface in &device.interfaces {
            if iface.usage_page == 0x01 && iface.usage == 0x06 {
                entries.push((
                    iface.path.clone(),
                    device.name.clone(),
                    Some(device.id.clone()),
                ));
            }
        }
    }
    input::set_source_map(entries);
}

#[tauri::command]
fn live_input_set(app: AppHandle, enabled: bool) -> input::LiveStatus {
    if enabled {
        refresh_live_sources(&app);
    }
    input::set_live(&app, enabled)
}

#[tauri::command]
fn live_input_status() -> input::LiveStatus {
    input::live_status()
}

#[tauri::command]
fn keyboard_sources(app: AppHandle) -> Vec<input::KeyboardSourceView> {
    let devices = app.state::<AppState>().devices.lock().unwrap().clone();
    let live = input::live_status();
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for device in devices.iter().filter(|d| d.online) {
        let keyboards: Vec<_> = device
            .interfaces
            .iter()
            .filter(|i| i.usage_page == 0x01 && i.usage == 0x06)
            .collect();
        if keyboards.is_empty() {
            continue;
        }
        for iface in &keyboards {
            seen.push(input::path_key(&iface.path));
        }
        let label = device.connection_label.to_ascii_lowercase();
        let receiver_shared = label.contains("2.4")
            || label.contains("receiver")
            || label.contains("dongle")
            || label.contains("ресивер")
            || label.contains("nearlink");
        out.push(input::KeyboardSourceView {
            device_id: Some(device.id.clone()),
            name: device.name.clone(),
            connection_label: device.connection_label.clone(),
            online: true,
            managed: !matches!(device.adapter_id.as_str(), "generic" | ""),
            receiver_shared,
            interfaces: keyboards.len(),
            paths: keyboards.iter().map(|iface| iface.path.clone()).collect(),
        });
    }
    for source in &live.sources {
        let path = input::path_key(&source.path);
        if seen.contains(&path) {
            continue;
        }
        out.push(input::KeyboardSourceView {
            device_id: source.device_id.clone(),
            name: source.label.clone(),
            connection_label: "Raw Input".into(),
            online: true,
            managed: false,
            receiver_shared: false,
            interfaces: 1,
            paths: vec![source.path.clone()],
        });
    }
    out
}

const OVERLAY_LABEL: &str = "overlay";
const OVERLAY_WIDTH: f64 = 620.0;
const OVERLAY_HEIGHT: f64 = 132.0;

#[cfg(windows)]
fn keep_overlay_passive(window: &WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    let Ok(raw) = window.hwnd() else { return };
    let hwnd = windows::Win32::Foundation::HWND(raw.0);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_NOACTIVATE.0 as isize);
    }
}

#[cfg(windows)]
fn restore_previous_foreground(window: &WebviewWindow, previous: windows::Win32::Foundation::HWND) {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};
    let Ok(raw) = window.hwnd() else { return };
    let own = raw.0 as isize;
    unsafe {
        if GetForegroundWindow().0 as isize != own {
            return;
        }
        if !previous.0.is_null() && previous.0 as isize != own {
            let _ = SetForegroundWindow(previous);
        }
    }
}

fn ensure_overlay_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        return Ok(window);
    }
    let window =
        WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App("index.html".into()))
            .title("UDM overlay")
            .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(false)
            .initialization_script("window.location.hash = '/overlay';")
            .build()?;
    let handle = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let was_visible = handle.is_visible().unwrap_or(false);
            let live = input::live_status().active;
            crate::store::log_line(&format!(
                "overlay close requested: visible={} live={}",
                was_visible, live
            ));
            let _ = handle.hide();
            if was_visible && live {
                let _ = input::set_live(handle.app_handle(), false);
            }
        }
    });
    Ok(window)
}

fn position_overlay(app: &AppHandle, window: &WebviewWindow, at_top: bool) {
    let Some(monitor) = app.primary_monitor().ok().flatten() else {
        let _ = window.center();
        return;
    };
    let work = monitor.work_area();
    let size = window.outer_size().unwrap_or(tauri::PhysicalSize::new(
        OVERLAY_WIDTH as u32,
        OVERLAY_HEIGHT as u32,
    ));
    let x = work.position.x + (work.size.width as i32 - size.width as i32) / 2;
    let y = if at_top {
        work.position.y + 48
    } else {
        work.position.y + work.size.height as i32 - size.height as i32 - 48
    };
    let _ = window.set_position(PhysicalPosition::new(
        x.max(work.position.x),
        y.max(work.position.y),
    ));
}

#[tauri::command]
async fn overlay_show(app: AppHandle) -> Result<(), String> {
    let window = ensure_overlay_window(&app).map_err(|e| e.to_string())?;
    position_overlay(&app, &window, false);
    let before = window.is_visible().unwrap_or(false);
    #[cfg(windows)]
    let foreground = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    window.show().map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        keep_overlay_passive(&window);
        restore_previous_foreground(&window, foreground);
    }
    if !input::live_status().active {
        refresh_live_sources(&app);
        input::set_live(&app, true);
    }
    let _ = app.emit_to(OVERLAY_LABEL, "overlay:refresh", ());
    crate::store::log_line(&format!("overlay shown: was_visible={}", before));
    Ok(())
}

#[tauri::command]
fn overlay_hide(app: AppHandle, stop_capture: Option<bool>) {
    let existed = if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        let _ = window.hide();
        true
    } else {
        false
    };
    let stop = stop_capture.unwrap_or(false);
    crate::store::log_line(&format!("overlay hide: existed={} stop={}", existed, stop));
    if stop && input::live_status().active {
        let _ = input::set_live(&app, false);
    }
}

#[tauri::command]
fn overlay_status(app: AppHandle) -> bool {
    app.get_webview_window(OVERLAY_LABEL)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

#[tauri::command]
async fn overlay_resize(
    app: AppHandle,
    width: f64,
    height: f64,
    position: Option<String>,
) -> Result<(), String> {
    let window = ensure_overlay_window(&app).map_err(|e| e.to_string())?;
    let width = width.clamp(360.0, 1280.0);
    let height = height.clamp(90.0, 420.0);
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    position_overlay(&app, &window, position.as_deref() == Some("top"));
    Ok(())
}

#[tauri::command]
fn autoclicker_start(
    app: AppHandle,
    config: autoclicker::AutoConfig,
) -> Result<autoclicker::AutoStatus, String> {
    autoclicker::start(app, config)
}

#[tauri::command]
fn autoclicker_stop() -> autoclicker::AutoStatus {
    autoclicker::stop()
}

#[tauri::command]
fn autoclicker_status() -> autoclicker::AutoStatus {
    autoclicker::status()
}

pub fn run() {
    let data_dir = std::env::var_os("APPDATA")
        .map(|p| std::path::PathBuf::from(p).join("com.udm.desktop"))
        .unwrap_or_else(|| std::path::PathBuf::from("UDM-data"));
    let builder = tauri::Builder::default().manage(AppState::new(data_dir));
    builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            crate::store::log_line("UDM starting");
            {
                use tauri_plugin_autostart::ManagerExt;
                let actual = app.autolaunch().is_enabled().unwrap_or(false);
                let state = app.state::<AppState>();
                let stored = state.settings();
                if stored.autostart != actual {
                    let mut fixed = stored.clone();
                    fixed.autostart = actual;
                    let _ = state.save_settings(&fixed);
                }
            }
            let handle = app.handle().clone();
            refresh_catalog(&handle);
            store_devices(&handle);
            macros::init(handle.clone());
            setup_tray(app)?;
            setup_window_behavior(app);
            macros::reload_hotkeys(&handle);
            input::start(handle.clone());
            let saved_map = {
                let state = app.state::<AppState>();
                let map = state.input_map.lock().unwrap().clone();
                map
            };
            input::apply(&saved_map);
            let game_mode = app.state::<AppState>().settings().game_mode;
            input::set_game_mode(game_mode);
            spawn_monitor(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_devices,
            rescan_devices,
            probe_battery,
            audio_list,
            audio_state,
            audio_set_volume,
            audio_set_channel,
            audio_set_mute,
            audio_sessions,
            audio_session_volume,
            audio_session_mute,
            open_sound_settings,
            eapo_status,
            eapo_save_preset,
            macros_list,
            macro_save,
            macro_delete,
            macro_record_start,
            macro_record_stop,
            macro_record_cancel,
            macro_play,
            macro_stop,
            macro_set_hotkey,
            profiles_list,
            profile_save,
            profile_delete,
            profile_activate,
            settings_get,
            settings_set,
            autostart_set,
            drivers_scan,
            driver_verify,
            drivers_history,
            driver_resolve,
            driver_install,
            driver_install_elevated,
            driver_store,
            firmware_info,
            catalog_search,
            catalog_download,
            driver_auto_download,
            system_info,
            log_tail,
            clipboard_copy,
            device_registry,
            db_info,
            db_history,
            db_backup,
            registry_bindings,
            registry_import,
            ble_inspect,
            diagnostics_export,
            mouse_status,
            mouse_set_dpi,
            mouse_set_polling,
            keyboard_status,
            keyboard_set_lighting,
            battery_rgb_get,
            battery_rgb_save,
            battery_rgb_enable,
            hid_read_feature,
            hid_write_feature,
            hid_descriptor,
            battery_history,
            tray_hide,
            tray_toggle,
            tray_open,
            input_status,
            input_map_set,
            input_enabled_set,
            mouse_system_get,
            mouse_system_set,
            key_catalog,
            polling_stats,
            polling_reset,
            telemetry_scan,
            telemetry_cached,
            protocol_feature_sweep,
            protocol_capture,
            live_input_set,
            live_input_status,
            keyboard_sources,
            overlay_show,
            overlay_hide,
            overlay_status,
            overlay_resize,
            autoclicker_start,
            autoclicker_stop,
            autoclicker_status,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
