use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::model::{BatteryInfo, BatteryRgbConfig, DeviceView, LightingSnapshot};
use crate::store::AppState;

static LIGHTING_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock() -> std::sync::MutexGuard<'static, ()> {
    LIGHTING_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatteryRgbView {
    pub config: BatteryRgbConfig,
    pub available: bool,
    pub percent: Option<u8>,
    pub charging: Option<bool>,
    pub battery_status: Option<String>,
    pub reason_ru: Option<String>,
    pub reason_en: Option<String>,
    pub restorable: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatteryRgbResult {
    pub ok: bool,
    pub applied_color: Option<String>,
    pub message: String,
    pub view: BatteryRgbView,
    pub status: Option<crate::alumix::KeyboardStatus>,
}

fn lang(state: &AppState) -> String {
    state.settings().language
}

fn pick(state: &AppState, ru: &str, en: &str) -> String {
    if lang(state) == "ru" {
        ru.to_string()
    } else {
        en.to_string()
    }
}

fn device_of(state: &AppState, device_id: &str) -> Result<DeviceView, String> {
    state
        .devices
        .lock()
        .unwrap()
        .iter()
        .find(|d| d.id == device_id)
        .cloned()
        .ok_or_else(|| "device not found".to_string())
}

fn config_of(state: &AppState, device_id: &str) -> BatteryRgbConfig {
    state
        .battery_rgb
        .lock()
        .unwrap()
        .get(device_id)
        .cloned()
        .unwrap_or_default()
}

fn store_config(
    state: &AppState,
    device_id: &str,
    config: &BatteryRgbConfig,
) -> Result<(), String> {
    state
        .battery_rgb
        .lock()
        .unwrap()
        .insert(device_id.to_string(), config.clone());
    state.save_battery_rgb()
}

fn parse_color(value: &str) -> Result<(), String> {
    let cleaned = value.trim().trim_start_matches('#');
    if cleaned.len() != 6 || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("color must be #RRGGBB".into());
    }
    Ok(())
}

fn brightness_level(value: u8) -> u8 {
    (((value.min(100) as f32) / 25.0).round() as u8) * 25
}

fn normalize(mut config: BatteryRgbConfig) -> Result<BatteryRgbConfig, String> {
    if config.ranges.is_empty() {
        return Err("Battery RGB: добавьте хотя бы один диапазон".into());
    }
    for range in config.ranges.iter_mut() {
        if range.min > range.max || range.max > 100 {
            return Err(format!(
                "Battery RGB: диапазон {}–{}% задан неверно",
                range.min, range.max
            ));
        }
        parse_color(&range.color)?;
        range.brightness = brightness_level(range.brightness);
    }
    if let Some(color) = config.charging_color.as_ref() {
        parse_color(color)?;
    }
    if let Some(value) = config.charging_brightness {
        config.charging_brightness = Some(brightness_level(value));
    }
    config.layout = "solid".into();
    config.ranges.sort_by_key(|range| range.min);
    let mut expected: u16 = 0;
    for range in &config.ranges {
        if range.min as u16 != expected {
            return Err(
                "Battery RGB: диапазоны должны покрывать 0–100% без пропусков и пересечений".into(),
            );
        }
        expected = range.max as u16 + 1;
    }
    if expected != 101 {
        return Err("Battery RGB: диапазоны должны покрывать 0–100%".into());
    }
    Ok(config)
}

fn battery_percent(device: &DeviceView) -> Option<u8> {
    let battery = device.battery.as_ref()?;
    let status = battery.status.as_str();
    if !matches!(status, "real" | "charging" | "estimated") {
        return None;
    }
    battery.percent.map(|value| value.min(100))
}

fn availability(device: &DeviceView) -> (bool, Option<String>, Option<String>) {
    if !device.online {
        return (
            false,
            Some("Устройство не подключено — Battery RGB ждёт переподключения.".into()),
            Some("The device is offline — Battery RGB waits for reconnection.".into()),
        );
    }
    if device.adapter_id != "redsquare" {
        return (
            false,
            Some(
                "RGB этого устройства не подтверждён, поэтому режим заряда не может менять подсветку."
                    .into(),
            ),
            Some(
                "RGB is not verified for this device, so the battery mode cannot change lighting."
                    .into(),
            ),
        );
    }
    match battery_percent(device) {
        Some(_) => (true, None, None),
        None => {
            let detail = device
                .battery
                .as_ref()
                .and_then(|battery| battery.note_ru.clone())
                .unwrap_or_else(|| "Устройство не отдаёт достоверный процент заряда.".to_string());
            (
                false,
                Some(format!(
                    "Battery RGB недоступен: нужен реальный процент заряда. {detail}"
                )),
                Some(
                    "Battery RGB is unavailable: a trustworthy battery percentage is required."
                        .into(),
                ),
            )
        }
    }
}

fn color_for(config: &BatteryRgbConfig, percent: u8, charging: bool) -> (String, u8) {
    if charging {
        if let Some(color) = config.charging_color.as_ref() {
            return (
                color.clone(),
                brightness_level(config.charging_brightness.unwrap_or(100)),
            );
        }
    }
    let range = config
        .ranges
        .iter()
        .find(|range| percent >= range.min && percent <= range.max)
        .or_else(|| config.ranges.last());
    match range {
        Some(range) => (range.color.clone(), brightness_level(range.brightness)),
        None => ("#ff00ff".into(), 100),
    }
}

fn snapshot_of(status: &crate::alumix::KeyboardStatus) -> LightingSnapshot {
    let direction = if status.effect_id == 1 {
        status.direction.min(1)
    } else {
        match status.direction {
            0 => 0,
            3 => 1,
            1 => 2,
            2 => 3,
            _ => 0,
        }
    };
    LightingSnapshot {
        effect: status.effect_id,
        brightness: brightness_level(status.brightness),
        speed: status.speed,
        direction,
        color: status.color.clone(),
    }
}

pub fn view(app: &AppHandle, device_id: &str) -> BatteryRgbView {
    let state = app.state::<AppState>();
    let config = config_of(&state, device_id);
    let device = device_of(&state, device_id).ok();
    let (available, reason_ru, reason_en) = match device.as_ref() {
        Some(device) => availability(device),
        None => (
            false,
            Some("Устройство не найдено.".into()),
            Some("Device not found.".into()),
        ),
    };
    BatteryRgbView {
        restorable: config.saved.is_some(),
        config,
        available,
        percent: device.as_ref().and_then(battery_percent),
        charging: device
            .as_ref()
            .and_then(|d| d.battery.as_ref().and_then(|b| b.charging)),
        battery_status: device
            .as_ref()
            .and_then(|d| d.battery.as_ref().map(|b| b.status.clone())),
        reason_ru,
        reason_en,
    }
}

pub fn save(
    app: &AppHandle,
    device_id: &str,
    config: BatteryRgbConfig,
) -> Result<BatteryRgbView, String> {
    let state = app.state::<AppState>();
    device_of(&state, device_id)?;
    let mut next = normalize(config)?;
    let previous = config_of(&state, device_id);
    next.saved = previous.saved;
    next.last_percent = previous.last_percent;
    next.last_color = previous.last_color;
    if next.enabled && !previous.enabled {
        next.enabled = false;
    }
    store_config(&state, device_id, &next)?;
    Ok(view(app, device_id))
}

pub fn set_enabled(
    app: &AppHandle,
    device_id: &str,
    enabled: bool,
) -> Result<BatteryRgbResult, String> {
    let _guard = lock();
    let state = app.state::<AppState>();
    let device = device_of(&state, device_id)?;
    let mut config = config_of(&state, device_id);
    config = normalize(config.clone()).unwrap_or(config);

    if !enabled {
        config.enabled = false;
        config.last_percent = None;
        config.last_color = None;
        let snapshot = config.saved.clone();
        if let Some(snapshot) = snapshot {
            if !device.online {
                store_config(&state, device_id, &config)?;
                let view = view(app, device_id);
                return Ok(BatteryRgbResult {
                    ok: true,
                    applied_color: None,
                    message: pick(
                        &state,
                        "Battery RGB выключен. Устройство offline — прошлая подсветка вернётся при подключении.",
                        "Battery RGB is off. The device is offline — the previous lighting will be restored on reconnect.",
                    ),
                    view,
                    status: None,
                });
            }
            match crate::alumix::set_lighting(
                &device,
                snapshot.effect,
                snapshot.brightness,
                snapshot.speed,
                snapshot.direction,
                &snapshot.color,
            ) {
                Ok(result) => {
                    let confirmed = result.ok;
                    if confirmed {
                        config.saved = None;
                    }
                    store_config(&state, device_id, &config)?;
                    crate::store::log_line(&format!(
                        "battery rgb disabled: {device_id}, restore confirmed={confirmed}"
                    ));
                    let view = view(app, device_id);
                    return Ok(BatteryRgbResult {
                        ok: confirmed,
                        applied_color: None,
                        message: if confirmed {
                            pick(
                                &state,
                                "Battery RGB выключен, прошлая подсветка восстановлена.",
                                "Battery RGB is off, the previous lighting has been restored.",
                            )
                        } else {
                            pick(
                                &state,
                                "Battery RGB выключен, но клавиатура не подтвердила возврат прошлой подсветки. Попробуйте ещё раз.",
                                "Battery RGB is off, but the keyboard did not confirm the restore. Try again.",
                            )
                        },
                        view,
                        status: result.status,
                    });
                }
                Err(error) => {
                    store_config(&state, device_id, &config)?;
                    let view = view(app, device_id);
                    return Ok(BatteryRgbResult {
                        ok: false,
                        applied_color: None,
                        message: pick(
                            &state,
                            &format!(
                                "Battery RGB выключен, но вернуть подсветку не удалось: {error}"
                            ),
                            &format!("Battery RGB is off, but restoring lighting failed: {error}"),
                        ),
                        view,
                        status: None,
                    });
                }
            }
        }
        store_config(&state, device_id, &config)?;
        return Ok(BatteryRgbResult {
            ok: true,
            applied_color: None,
            message: pick(&state, "Battery RGB выключен.", "Battery RGB is off."),
            view: view(app, device_id),
            status: None,
        });
    }

    let (available, reason_ru, reason_en) = availability(&device);
    if !available {
        return Ok(BatteryRgbResult {
            ok: false,
            applied_color: None,
            message: pick(
                &state,
                reason_ru.as_deref().unwrap_or("Battery RGB недоступен."),
                reason_en
                    .as_deref()
                    .unwrap_or("Battery RGB is unavailable."),
            ),
            view: view(app, device_id),
            status: None,
        });
    }

    config = normalize(config)?;
    let percent = battery_percent(&device).ok_or("battery percent is unavailable")?;
    let charging = device
        .battery
        .as_ref()
        .and_then(|battery| battery.charging)
        .unwrap_or(false);

    let current = crate::alumix::read_status(&device)
        .map_err(|error| format!("Battery RGB: не удалось прочитать текущую подсветку: {error}"))?;
    let snapshot = snapshot_of(&current);
    let (color, brightness) = color_for(&config, percent, charging);

    let result = crate::alumix::set_lighting(&device, 8, brightness, 0, 0, &color)?;
    if !result.ok {
        return Ok(BatteryRgbResult {
            ok: false,
            applied_color: None,
            message: pick(
                &state,
                &format!(
                    "Клавиатура не подтвердила индикацию заряда: {}",
                    result.message
                ),
                &format!(
                    "The keyboard did not confirm the battery indication: {}",
                    result.message
                ),
            ),
            view: view(app, device_id),
            status: result.status,
        });
    }

    config.enabled = true;
    config.saved = Some(snapshot);
    config.last_percent = Some(percent);
    config.last_color = Some(color.clone());
    store_config(&state, device_id, &config)?;
    crate::store::log_line(&format!(
        "battery rgb enabled: {device_id}, percent={percent}, color={color}, brightness={brightness}"
    ));
    Ok(BatteryRgbResult {
        ok: true,
        applied_color: Some(color.clone()),
        message: pick(
            &state,
            &format!("Battery RGB включён: {percent}% → {color}."),
            &format!("Battery RGB is on: {percent}% → {color}."),
        ),
        view: view(app, device_id),
        status: result.status,
    })
}

pub fn apply_if_enabled(app: &AppHandle, device_id: &str, battery: &BatteryInfo) {
    let state = app.state::<AppState>();
    let config = config_of(&state, device_id);
    if !config.enabled {
        return;
    }
    let Some(percent) = battery
        .percent
        .map(|value| value.min(100))
        .filter(|_| matches!(battery.status.as_str(), "real" | "charging" | "estimated"))
    else {
        return;
    };
    let charging = battery.charging.unwrap_or(false);
    let (color, brightness) = color_for(&config, percent, charging);
    if config.last_percent == Some(percent) && config.last_color.as_deref() == Some(color.as_str())
    {
        return;
    }
    let Ok(device) = device_of(&state, device_id) else {
        return;
    };
    if !device.online || device.adapter_id != "redsquare" {
        return;
    }
    let _guard = lock();
    match crate::alumix::set_lighting(&device, 8, brightness, 0, 0, &color) {
        Ok(result) if result.ok => {
            let mut next = config_of(&state, device_id);
            next.last_percent = Some(percent);
            next.last_color = Some(color.clone());
            let _ = store_config(&state, device_id, &next);
            crate::store::log_line(&format!(
                "battery rgb auto: {device_id}, percent={percent}, color={color}"
            ));
            let _ = app.emit(
                "battery-rgb:applied",
                serde_json::json!({
                    "deviceId": device_id,
                    "percent": percent,
                    "color": color,
                }),
            );
        }
        Ok(result) => {
            crate::store::log_line(&format!(
                "battery rgb auto not confirmed: {device_id}, {}",
                result.message
            ));
        }
        Err(error) => {
            crate::store::log_line(&format!("battery rgb auto failed: {device_id}, {error}"));
        }
    }
}
