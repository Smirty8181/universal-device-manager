use std::ffi::CString;
use std::path::Path;

use hidapi::HidApi;

use crate::model::{DeviceView, HidProbeResult};

fn ensure_path(device: &DeviceView, path: &str) -> Result<(), String> {
    if device.interfaces.iter().any(|i| i.path == path) {
        Ok(())
    } else {
        Err("interface path does not belong to the selected device".into())
    }
}

fn hex_bytes(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_hex(input: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',' && *c != '-')
        .collect();
    if cleaned.is_empty() || cleaned.len() % 2 != 0 {
        return Err("hex payload must contain whole bytes".into());
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    for i in (0..cleaned.len()).step_by(2) {
        let byte = u8::from_str_radix(&cleaned[i..i + 2], 16)
            .map_err(|_| "invalid hex payload".to_string())?;
        out.push(byte);
    }
    if out.len() > 1024 {
        return Err("payload is too long for a feature report".into());
    }
    Ok(out)
}

pub fn read_feature(
    device: &DeviceView,
    path: &str,
    report_id: u8,
    length: usize,
) -> Result<HidProbeResult, String> {
    ensure_path(device, path)?;
    if length == 0 || length > 4096 {
        return Err("length must be between 1 and 4096".into());
    }
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let cpath = CString::new(path).map_err(|_| "invalid interface path".to_string())?;
    let handle = api.open_path(&cpath).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; length + 1];
    buf[0] = report_id;
    match handle.get_feature_report(&mut buf) {
        Ok(read) => Ok(HidProbeResult {
            data: hex_bytes(&buf[..read.max(1)]),
            error: None,
        }),
        Err(e) => Ok(HidProbeResult {
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

pub fn write_feature(device: &DeviceView, path: &str, hex: &str) -> Result<HidProbeResult, String> {
    ensure_path(device, path)?;
    let payload = parse_hex(hex)?;
    if payload.len() < 2 {
        return Err("feature report must start with a report ID and contain data".into());
    }
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let cpath = CString::new(path).map_err(|_| "invalid interface path".to_string())?;
    let handle = api.open_path(&cpath).map_err(|e| e.to_string())?;
    match handle.send_feature_report(&payload) {
        Ok(()) => {
            crate::store::log_line(&format!(
                "HID feature write: device={} path={} bytes={}",
                device.id,
                path,
                payload.len()
            ));
            Ok(HidProbeResult {
                data: format!("sent {} bytes", payload.len()),
                error: None,
            })
        }
        Err(e) => Ok(HidProbeResult {
            data: String::new(),
            error: Some(e.to_string()),
        }),
    }
}

pub fn descriptor(device: &DeviceView) -> Result<String, String> {
    let Some(interface) = device
        .interfaces
        .iter()
        .find(|i| i.usage_page >= 0xFF00)
        .or_else(|| device.interfaces.first())
    else {
        return Err("device has no HID interfaces".into());
    };
    match crate::hid_desc::caps_for_path(&interface.path) {
        Ok(caps) => Ok(format!(
            "{}\nКоллекции HID:\n{}",
            crate::hid_desc::caps_text(&caps),
            collections_text(device)
        )),
        Err(error) => Ok(format!(
            "Windows не отдала preparsed-данные: {error}\nКоллекции HID:\n{}",
            collections_text(device)
        )),
    }
}

fn collections_text(device: &DeviceView) -> String {
    if device.interfaces.is_empty() {
        return "—".into();
    }
    device
        .interfaces
        .iter()
        .map(|i| {
            format!(
                "0x{:04X}:0x{:02X} · MI {} · {}",
                i.usage_page,
                i.usage,
                i.interface_number.unwrap_or(-1),
                Path::new(&i.path)
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
