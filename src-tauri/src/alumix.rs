use std::ffi::CString;
use std::thread::sleep;
use std::time::Duration;

use hidapi::HidApi;
use serde::Serialize;

use crate::model::DeviceView;

const REPORT_ID: u8 = 7;
const READ_LEN: usize = 264;
const WRITE_LEN: usize = 256;

const EFFECTS: [(u8, u8); 15] = [
    (0, 0),
    (1, 6),
    (2, 2),
    (3, 7),
    (4, 4),
    (5, 8),
    (6, 1),
    (7, 3),
    (8, 5),
    (9, 9),
    (10, 10),
    (11, 11),
    (12, 12),
    (13, 13),
    (19, 14),
];

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardStatus {
    pub path_kind: String,
    pub profile_index: u8,
    pub effect_id: u8,
    pub hid_effect: u8,
    pub brightness: u8,
    pub speed: u8,
    pub direction: u8,
    pub multi_color: bool,
    pub color: String,
    pub custom_index: u8,
    pub polling_hz: Option<u32>,
    pub latency_ms: Option<u8>,
    pub sleep_minutes: Option<u8>,
    pub debounce_ms: Option<u8>,
    pub battery_raw: Option<u8>,
    pub battery: Option<u8>,
    pub firmware: Option<String>,
    pub note_ru: Option<String>,
    pub note_en: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LightingCommandResult {
    pub ok: bool,
    pub status: Option<KeyboardStatus>,
    pub message: String,
}

fn config_candidates(device: &DeviceView) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (page, usage, kind) in [(0xFF01, 0x01, "usb"), (0xFF60, 0x61, "dongle")] {
        if let Some(interface) = device
            .interfaces
            .iter()
            .find(|i| i.usage_page == page && i.usage == usage)
        {
            out.push((interface.path.clone(), kind.into()));
        }
    }
    for interface in &device.interfaces {
        if interface.usage_page < 0xFF00 || interface.path.ends_with("\\KBD") {
            continue;
        }
        if out.iter().any(|(path, _)| path == &interface.path) {
            continue;
        }
        out.push((interface.path.clone(), "usb".into()));
    }
    out
}

fn open(path: &str) -> Result<hidapi::HidDevice, String> {
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let cpath = CString::new(path).map_err(|_| "invalid interface path".to_string())?;
    api.open_path(&cpath).map_err(|e| e.to_string())
}

fn open_config(device: &DeviceView) -> Result<(hidapi::HidDevice, String, Vec<u8>), String> {
    let candidates = config_candidates(device);
    if candidates.is_empty() {
        return Err("vendor config interface not found".into());
    }
    let mut dongle_seen = false;
    let mut last_error = String::new();
    for (path, kind) in candidates {
        if kind == "dongle" {
            dongle_seen = true;
        }
        let handle = match open(&path) {
            Ok(handle) => handle,
            Err(error) => {
                last_error = error;
                continue;
            }
        };
        match read_command(&handle, 0x81, 0) {
            Ok(buf) if buf.get(1) == Some(&0x81) => return Ok((handle, kind, buf)),
            Ok(_) => last_error = "0x81 reply did not match".into(),
            Err(error) => last_error = error,
        }
    }
    if dongle_seen {
        Err("Red Square: 2.4G-ресивер не ответил на vendor-команды. Подключите клавиатуру USB-кабелем для подсветки и заряда. / The 2.4G receiver did not answer vendor commands. Connect the keyboard by USB cable for lighting and charge.".into())
    } else {
        Err(format!(
            "vendor config interface did not answer: {last_error}"
        ))
    }
}

fn read_settings(handle: &hidapi::HidDevice) -> Option<Vec<u8>> {
    let mut last = None;
    for attempt in 0..4 {
        if attempt > 0 {
            sleep(Duration::from_millis(140));
        }
        if let Ok(buf) = read_command(handle, 0x8B, 0) {
            if buf.get(1) == Some(&0x8B) {
                let valid = buf
                    .get(4)
                    .map(|value| (1..=100).contains(value))
                    .unwrap_or(false);
                last = Some(buf);
                if valid {
                    break;
                }
            }
        }
    }
    last
}

fn read_command(handle: &hidapi::HidDevice, command: u8, argument: u8) -> Result<Vec<u8>, String> {
    let mut out = vec![0u8; READ_LEN + 1];
    out[0] = REPORT_ID;
    out[1] = command;
    out[2] = argument;
    handle
        .send_feature_report(&out)
        .map_err(|e| format!("send 0x{command:02X}: {e}"))?;
    sleep(Duration::from_millis(110));
    let mut buf = vec![0u8; READ_LEN + 1];
    buf[0] = REPORT_ID;
    let read = handle
        .get_feature_report(&mut buf)
        .map_err(|e| format!("read 0x{command:02X}: {e}"))?;
    buf.truncate(read.max(1));
    if buf.len() < 90 {
        return Err(format!("short feature report for 0x{command:02X}"));
    }
    Ok(buf)
}

fn write_command(handle: &hidapi::HidDevice, payload: &[u8]) -> Result<(), String> {
    if payload.len() != WRITE_LEN {
        return Err("lighting payload must be 256 bytes".into());
    }
    let mut out = vec![0u8; WRITE_LEN + 1];
    out[0] = REPORT_ID;
    out[1..].copy_from_slice(payload);
    handle
        .send_feature_report(&out)
        .map_err(|e| format!("lighting write: {e}"))
}

fn effect_id_from_hid(hid: u8) -> u8 {
    EFFECTS
        .iter()
        .find(|(_, h)| *h == hid)
        .map(|(id, _)| *id)
        .unwrap_or(hid)
}

fn hid_from_effect(effect: u8) -> Option<u8> {
    EFFECTS
        .iter()
        .find(|(id, _)| *id == effect)
        .map(|(_, hid)| *hid)
}

fn color_of(buf: &[u8]) -> String {
    format!("#{:02x}{:02x}{:02x}", buf[59], buf[67], buf[75])
}

fn parse_hex_color(value: &str) -> Result<(u8, u8, u8), String> {
    let cleaned = value.trim().trim_start_matches('#');
    if cleaned.len() != 6 || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("color must be #RRGGBB".into());
    }
    let r = u8::from_str_radix(&cleaned[0..2], 16).map_err(|_| "invalid color".to_string())?;
    let g = u8::from_str_radix(&cleaned[2..4], 16).map_err(|_| "invalid color".to_string())?;
    let b = u8::from_str_radix(&cleaned[4..6], 16).map_err(|_| "invalid color".to_string())?;
    Ok((r, g, b))
}

struct ProfileLayout {
    effect: u8,
    hid: u8,
    brightness: u8,
    speed: u8,
    direction: u8,
    multi: bool,
    color: String,
    custom_index: u8,
    polling: Option<u32>,
    latency: Option<u8>,
}

fn parse_profile(buf: &[u8]) -> Option<ProfileLayout> {
    if buf.len() < 90 {
        return None;
    }
    let hid = buf[9];
    if hid > 30 {
        return None;
    }
    let effect = effect_id_from_hid(hid);
    let brightness = (buf[10 + hid as usize] as u32 * 25).min(100) as u8;
    let speed_level = buf[24 + hid as usize].min(5);
    let speed = (5 - speed_level) * 25;
    let direction = if effect == 1 { buf[84] } else { buf[52] };
    let multi = buf[37 + hid as usize] == 8;
    let color = color_of(buf);
    let custom_index = buf[53].saturating_sub(1);
    let polling = match buf[8] {
        0 => Some(125),
        1 => Some(250),
        2 => Some(500),
        3 => Some(1000),
        _ => None,
    };
    let latency = match buf[88] {
        0 => Some(0),
        1 => Some(2),
        2 => Some(8),
        3 => Some(12),
        _ => None,
    };
    Some(ProfileLayout {
        effect,
        hid,
        brightness,
        speed,
        direction,
        multi,
        color,
        custom_index,
        polling,
        latency,
    })
}

pub fn read_status(device: &DeviceView) -> Result<KeyboardStatus, String> {
    if device.adapter_id != "redsquare" {
        return Err("not a Red Square device".into());
    }
    let (handle, kind, current) = open_config(device)?;
    let profile_index = match current.get(2) {
        Some(value) if (1..=3).contains(value) => *value,
        _ => 1,
    };
    let mut parsed = read_command(&handle, 0x82, profile_index)
        .ok()
        .and_then(|buf| parse_profile(&buf));
    if parsed.is_none() {
        for index in 1..=3u8 {
            if index == profile_index {
                continue;
            }
            parsed = read_command(&handle, 0x82, index)
                .ok()
                .and_then(|buf| parse_profile(&buf));
            if parsed.is_some() {
                break;
            }
        }
    }
    let ProfileLayout {
        effect,
        hid,
        brightness,
        speed,
        direction,
        multi,
        color,
        custom_index,
        polling,
        latency,
    } = parsed.ok_or("unexpected profile layout")?;
    let settings = read_settings(&handle);
    let version = read_command(&handle, 0x88, 0).ok();
    let sleep_minutes = settings.as_ref().and_then(|s| s.get(2).copied());
    let debounce_ms = settings.as_ref().and_then(|s| s.get(3).copied());
    let battery_raw = settings.as_ref().and_then(|s| s.get(4).copied());
    let battery = battery_raw.filter(|v| (1..=100).contains(v));
    let firmware = version.as_ref().map(|v| {
        let value = ((v[14] & 0x0F) as u16) * 256 + v[15] as u16;
        format!("0x{value:02X}")
    });
    let note_ru = if battery.is_some() {
        Some(if kind == "dongle" {
            "Заряд прочитан с 2.4G-ресивера командой 0x8B.".to_string()
        } else {
            "Заряд прочитан командой 0x8B.".to_string()
        })
    } else if kind == "dongle" {
        Some(
            "2.4G-ресивер не вернул процент заряда по 0x8B; подключите клавиатуру USB-кабелем."
                .to_string(),
        )
    } else {
        Some("Команда 0x8B не вернула процент заряда; значение не подменяется оценкой.".to_string())
    };
    crate::store::log_line(&format!(
        "alumix status: profile={} effect={} brightness={} speed={} polling={:?} battery_raw={:?}",
        profile_index, effect, brightness, speed, polling, battery_raw
    ));
    Ok(KeyboardStatus {
        path_kind: kind,
        profile_index,
        effect_id: effect,
        hid_effect: hid,
        brightness,
        speed,
        direction,
        multi_color: multi,
        color,
        custom_index,
        polling_hz: polling,
        latency_ms: latency,
        sleep_minutes,
        debounce_ms,
        battery_raw,
        battery,
        firmware,
        note_ru,
        note_en: Some(
            "Lighting is read over the official Red Square protocol (feature report 7).".into(),
        ),
    })
}

pub fn set_lighting(
    device: &DeviceView,
    effect: u8,
    brightness: u8,
    speed: u8,
    direction: u8,
    color: &str,
) -> Result<LightingCommandResult, String> {
    if device.adapter_id != "redsquare" {
        return Err("not a Red Square device".into());
    }
    let hid = hid_from_effect(effect)
        .ok_or_else(|| format!("effect {effect} is not supported by Alumix"))?;
    if brightness > 100 || brightness % 25 != 0 {
        return Err("brightness must be 0, 25, 50, 75 or 100".into());
    }
    if speed > 100 || speed % 25 != 0 {
        return Err("speed must be 0, 25, 50, 75 or 100".into());
    }
    if direction > 3 {
        return Err("direction must be 0..3".into());
    }
    let (r, g, b) = parse_hex_color(color)?;
    let (handle, _kind, _current) = open_config(device)?;

    let status = read_status(device)?;
    let profile = read_command(&handle, 0x82, status.profile_index)?;
    let mut payload = vec![0u8; WRITE_LEN];
    payload.copy_from_slice(&profile[1..WRITE_LEN + 1]);

    payload[0] = 0x02;
    payload[8] = hid;
    payload[9 + hid as usize] = (brightness as u32 * 4).div_ceil(100) as u8;
    payload[23 + hid as usize] = (5.0 - 4.0 * speed as f32 / 100.0).round() as u8;
    payload[37 + hid as usize] = 0;
    let direction_byte = if effect == 1 {
        direction.min(1)
    } else {
        match direction {
            0 => 0,
            1 => 3,
            2 => 1,
            _ => 2,
        }
    };
    payload[if effect == 1 { 83 } else { 51 }] = direction_byte;
    payload[82] = if effect == 7 { 0 } else { 8 };
    payload[85] = status.debounce_ms.unwrap_or(5);
    payload[52] = if effect == 19 {
        status.custom_index.saturating_add(1)
    } else {
        0
    };
    payload[58] = r;
    payload[66] = g;
    payload[74] = b;
    write_command(&handle, &payload)?;
    crate::store::log_line(&format!(
        "alumix lighting write: effect={effect} brightness={brightness} speed={speed} direction={direction} color=#{r:02x}{g:02x}{b:02x}"
    ));

    // The first profile read after a write returns the previous state on this firmware.
    sleep(Duration::from_millis(260));
    let _ = read_command(&handle, 0x82, status.profile_index);
    sleep(Duration::from_millis(160));
    let verified = read_command(&handle, 0x82, status.profile_index)
        .ok()
        .and_then(|buf| parse_profile(&buf));
    let confirmed = verified
        .as_ref()
        .map(|layout| {
            layout.effect == effect
                && layout.brightness == brightness
                && layout.speed == speed
                && layout.color == format!("#{r:02x}{g:02x}{b:02x}")
        })
        .unwrap_or(false);
    let fresh = read_status(device).ok();
    Ok(LightingCommandResult {
        ok: confirmed,
        status: fresh,
        message: if confirmed {
            format!("Эффект {effect} применён и подтверждён чтением")
        } else {
            "Команда отправлена, но клавиатура не подтвердила новые значения".into()
        },
    })
}
