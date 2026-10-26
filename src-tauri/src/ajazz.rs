use std::ffi::CString;
use std::thread::sleep;
use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};
use serde::Serialize;

use crate::model::DeviceView;

const REQUEST: [u8; 6] = [0x06, 0x05, 0x01, 0x00, 0x00, 0x00];
const REPORT_LEN: usize = 64;
const HALF_MS: i32 = 250;

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct MouseStatus {
    pub battery: Option<u8>,
    pub charging: Option<bool>,
    pub dpi: Vec<u16>,
    pub active_stage: Option<usize>,
    pub polling_hz: Option<u32>,
    pub source: String,
    pub raw: Option<String>,
    pub note_ru: Option<String>,
    pub note_en: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseCommandResult {
    pub ok: bool,
    pub status: Option<MouseStatus>,
    pub message: String,
}

fn vendor_paths(device: &DeviceView) -> Vec<String> {
    let mut ranked: Vec<(u8, String)> = device
        .interfaces
        .iter()
        .filter_map(|interface| {
            // 0xFFB2 is the collection that accepts output reports; the other
            // two open read-only on Windows and reject writes.
            let rank = match interface.usage_page {
                0xFFB2 => 0,
                0xFFB1 => 1,
                0xFFB3 => 2,
                _ => return None,
            };
            Some((rank, interface.path.clone()))
        })
        .collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked.into_iter().map(|(_, path)| path).collect()
}

fn error_text(error: hidapi::HidError) -> String {
    let text = error.to_string();
    if text.trim_end() == "hidapi error:" {
        "Windows rejected the HID request without an error code".to_string()
    } else {
        text
    }
}

fn open(api: &HidApi, path: &str) -> Result<HidDevice, String> {
    let cpath = CString::new(path).map_err(|_| "invalid interface path".to_string())?;
    api.open_path(&cpath).map_err(error_text)
}

fn drain(handle: &HidDevice) {
    let mut buf = [0u8; REPORT_LEN];
    for _ in 0..16 {
        match handle.read_timeout(&mut buf, 10) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

fn padded(frame: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; REPORT_LEN];
    let copy = frame.len().min(REPORT_LEN);
    buf[..copy].copy_from_slice(&frame[..copy]);
    buf
}

// Reply to `06 05 01`: [06][05][len][00][00][type] + value, where the value
// carries the PID at 1, battery at 7, stage count at 10, active stage at 11,
// six encoded DPI stages at 12 and the report-rate index at 24.
fn parse_block(frame: &[u8]) -> Option<MouseStatus> {
    if frame.len() < 32 || frame[0] != 0x06 || frame[1] != 0x05 {
        return None;
    }
    let value_len = frame[2].saturating_sub(1) as usize;
    if value_len < 25 || frame.len() < value_len + 6 {
        return None;
    }
    let value = &frame[6..value_len + 6];
    if value[7] > 100 {
        return None;
    }
    let battery = match value[7] {
        0 => None,
        percent => Some(percent),
    };
    let stage_count = (value[10] as usize).clamp(1, 8);
    let mut dpi = Vec::with_capacity(stage_count);
    for index in 0..stage_count {
        let offset = 12 + index * 2;
        if offset + 1 >= value.len() {
            return None;
        }
        let raw = u16::from_le_bytes([value[offset], value[offset + 1]]) as u32;
        let dpi_value = (raw + 1) * 50;
        if !(100..=60_000).contains(&dpi_value) {
            return None;
        }
        dpi.push(dpi_value as u16);
    }
    let active_stage = (value[11] as usize)
        .lt(&stage_count)
        .then_some(value[11] as usize);
    let polling_hz = match value[24] {
        0 => Some(125),
        1 => Some(250),
        2 => Some(500),
        3 => Some(1000),
        _ => None,
    };
    Some(MouseStatus {
        battery,
        charging: None,
        dpi,
        active_stage,
        polling_hz,
        source: "vendor-hid".into(),
        raw: Some(
            frame
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" "),
        ),
        note_ru: Some("Прочитано с мыши по протоколу NearLink (блок 06 05).".into()),
        note_en: Some("Read from the mouse over the NearLink protocol (06 05 block).".into()),
    })
}

// Notifications: [06][FA][len][00][00][tag] + value. Only tag 0x03 is the
// battery; tag 0x01 is a link notification, tag 0x02 a DPI notification.
fn parse_push(frame: &[u8]) -> Option<(u8, bool)> {
    if frame.len() >= 8
        && frame[0] == 0x06
        && frame[1] == 0xFA
        && frame[5] == 0x03
        && frame[6] <= 100
    {
        return Some((frame[6], frame[7] != 0));
    }
    None
}

fn parse_link(frame: &[u8]) -> Option<u8> {
    if frame.len() >= 7 && frame[0] == 0x06 && frame[1] == 0xFA && frame[5] == 0x01 {
        Some(frame[6])
    } else {
        None
    }
}

fn request(handle: &HidDevice) -> Result<(), String> {
    handle
        .write(&padded(&REQUEST))
        .map(|_| ())
        .map_err(error_text)
}

pub fn read_status(device: &DeviceView) -> Result<MouseStatus, String> {
    if device.adapter_id != "ajazz" {
        return Err("not an AJAZZ device".into());
    }
    let paths = vendor_paths(device);
    if paths.is_empty() {
        return Err("vendor HID interface not found".into());
    }
    let api = HidApi::new().map_err(error_text)?;
    let mut last_error = String::from("no reply from vendor interface");
    let mut no_reply_error: Option<String> = None;
    for path in paths {
        let handle = match open(&api, &path) {
            Ok(handle) => handle,
            Err(error) => {
                last_error = error;
                continue;
            }
        };
        drain(&handle);
        let mut battery_push = None;
        let mut link_seen = false;
        let mut block_status = None;
        let mut wrote = false;
        for attempt in 0..2 {
            if let Err(error) = request(&handle) {
                last_error = error;
                break;
            }
            wrote = true;
            sleep(Duration::from_millis(150));
            let mut buf = [0u8; REPORT_LEN];
            let deadline = Instant::now() + Duration::from_millis(850);
            while Instant::now() < deadline {
                match handle.read_timeout(&mut buf, HALF_MS) {
                    Ok(read) if read >= 8 => {
                        let frame = &buf[..read];
                        if let Some((percent, charging)) = parse_push(frame) {
                            battery_push = Some((percent, charging));
                        }
                        if parse_link(frame).is_some() {
                            link_seen = true;
                        }
                        if block_status.is_none() {
                            block_status = parse_block(frame);
                        }
                    }
                    Ok(_) => {}
                    Err(_) => {}
                }
                if block_status.is_some() {
                    break;
                }
            }
            if block_status.is_some() {
                break;
            }
            if attempt == 0 {
                sleep(Duration::from_millis(150));
            }
        }
        if let Some(mut status) = block_status {
            if let Some((percent, charging)) = battery_push {
                if status.battery.is_none() {
                    status.battery = Some(percent);
                }
                status.charging = Some(charging);
            }
            crate::store::log_line(&format!(
                "ajazz status: battery={:?} dpi={:?} rate={:?}",
                status.battery, status.dpi, status.polling_hz
            ));
            return Ok(status);
        }
        if let Some((percent, charging)) = battery_push {
            crate::store::log_line(&format!(
                "ajazz push: battery={percent} charging={charging} (no 06 05 block)"
            ));
            return Ok(MouseStatus {
                battery: Some(percent),
                charging: Some(charging),
                dpi: Vec::new(),
                active_stage: None,
                polling_hz: None,
                source: "vendor-hid-push".into(),
                raw: None,
                note_ru: Some(
                    "Батарея пришла push-кадром 06 FA 03; таблица DPI в нём отсутствует.".into(),
                ),
                note_en: Some(
                    "Battery came from the 06 FA 03 push frame; it carries no DPI table.".into(),
                ),
            });
        }
        if wrote {
            crate::store::log_line(if link_seen {
                "ajazz probe: receiver answered the link notification but not the 06 05 request"
            } else {
                "ajazz probe: request was written but no 06 05 block came back"
            });
            no_reply_error = Some(if link_seen {
                "Мышь не отвечает: похоже, она спит или выключена — разбудите её и повторите. / The mouse is not replying: it is probably asleep or switched off — wake it and retry.".into()
            } else {
                "Мышь приняла команду, но не вернула блок статуса 06 05 — повторите. / The mouse accepted the command but returned no 06 05 status block — try again.".into()
            });
        }
    }
    Err(no_reply_error.unwrap_or(last_error))
}

fn verify(device: &DeviceView) -> Option<MouseStatus> {
    sleep(Duration::from_millis(250));
    read_status(device).ok()
}

fn write_frame(device: &DeviceView, frame: &[u8]) -> Result<(), String> {
    let paths = vendor_paths(device);
    if paths.is_empty() {
        return Err("vendor HID interface not found".into());
    }
    let api = HidApi::new().map_err(error_text)?;
    let mut last_error = String::from("no writable vendor collection");
    for path in paths {
        let handle = match open(&api, &path) {
            Ok(handle) => handle,
            Err(error) => {
                last_error = error;
                continue;
            }
        };
        match handle.write(&padded(frame)) {
            Ok(_) => return Ok(()),
            Err(error) => last_error = error_text(error),
        }
    }
    Err(last_error)
}

pub fn set_dpi(device: &DeviceView, stage: usize, dpi: u32) -> Result<MouseCommandResult, String> {
    if device.adapter_id != "ajazz" {
        return Err("not an AJAZZ device".into());
    }
    if stage > 5 {
        return Err("DPI stage must be 0..5".into());
    }
    if !(100..=30_000).contains(&dpi) || dpi % 50 != 0 {
        return Err("DPI must be 100..30000 in steps of 50".into());
    }
    let [lo, hi] = (dpi as u16).to_le_bytes();
    let frame = [
        0x06,
        0x13,
        0x05,
        0x00,
        0x00,
        0x01,
        0x06,
        stage as u8,
        lo,
        hi,
        0x00,
    ];
    write_frame(device, &frame)?;
    crate::store::log_line(&format!("ajazz dpi write: stage={} dpi={}", stage, dpi));
    let status = verify(device);
    let confirmed = status
        .as_ref()
        .and_then(|s| s.dpi.get(stage).copied())
        .map(|value| value as u32 == dpi)
        .unwrap_or(false);
    Ok(MouseCommandResult {
        ok: confirmed,
        status,
        message: if confirmed {
            format!("DPI stage {} = {} (подтверждено чтением)", stage + 1, dpi)
        } else {
            "Команда отправлена, но чтение не подтвердило значение".into()
        },
    })
}

pub fn set_polling(device: &DeviceView, hz: u32) -> Result<MouseCommandResult, String> {
    if device.adapter_id != "ajazz" {
        return Err("not an AJAZZ device".into());
    }
    if ![125, 250, 500, 1000].contains(&hz) {
        return Err("supported polling rates: 125, 250, 500, 1000".into());
    }
    let rate_index: u8 = match hz {
        125 => 0,
        250 => 1,
        500 => 2,
        _ => 3,
    };
    let frame = [0x06, 0x13, 0x02, 0x00, 0x00, 0x02, rate_index, 0x00];
    write_frame(device, &frame)?;
    crate::store::log_line(&format!("ajazz polling write: {hz} Hz"));
    let status = verify(device);
    let confirmed = status.as_ref().and_then(|s| s.polling_hz) == Some(hz);
    Ok(MouseCommandResult {
        ok: confirmed,
        status,
        message: if confirmed {
            format!("Частота опроса {} Hz (подтверждено чтением)", hz)
        } else {
            "Команда отправлена, но чтение не подтвердило частоту".into()
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(body: &[u8]) -> Vec<u8> {
        let mut buf = vec![0u8; REPORT_LEN];
        buf[..body.len()].copy_from_slice(body);
        buf
    }

    #[test]
    fn parses_info_block_with_documented_offsets() {
        let raw = [
            0x06, 0x05, 0x24, 0x00, 0x00, 0x01, 0x00, 0x26, 0x6D, 0x0A, 0x10, 0x01, 0x01, 0x64,
            0x00, 0x00, 0x06, 0x00, 0x17, 0x00, 0x0F, 0x00, 0x13, 0x00, 0x1F, 0x00, 0x2F, 0x00,
            0x3F, 0x00, 0x02,
        ];
        let status = parse_block(&frame(&raw)).expect("info block");
        assert_eq!(status.battery, Some(100));
        assert_eq!(status.dpi, vec![1200, 800, 1000, 1600, 2400, 3200]);
        assert_eq!(status.active_stage, Some(0));
        assert_eq!(status.polling_hz, Some(500));
    }

    #[test]
    fn rejects_frame_with_implausible_battery() {
        let mut raw = [0u8; 31];
        raw[0] = 0x06;
        raw[1] = 0x05;
        raw[2] = 0x24;
        raw[5] = 0x01;
        raw[6 + 7] = 0xFF;
        assert!(parse_block(&frame(&raw)).is_none());
    }

    #[test]
    fn reads_only_battery_pushes() {
        let battery = frame(&[0x06, 0xFA, 0x03, 0x00, 0x00, 0x03, 0x64, 0x01]);
        assert_eq!(parse_push(&battery), Some((100, true)));
        let link = frame(&[0x06, 0xFA, 0x02, 0x00, 0x00, 0x01, 0x00]);
        assert_eq!(parse_push(&link), None);
        assert_eq!(parse_link(&link), Some(0));
    }
}
