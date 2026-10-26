use crate::adapters;
use crate::ajazz;
use crate::alumix;
use crate::ble;
use crate::hid_desc::{self, HidField};
use crate::model::{now_ms, BatteryInfo, DeviceView, HidInterface};

#[derive(Default, Clone)]
pub struct Detection {
    pub hid: Option<(String, HidField)>,
    pub ble: bool,
}

pub fn detect(interfaces: &[HidInterface], bluetooth_address: Option<&str>) -> Detection {
    let mut detection = Detection {
        hid: None,
        ble: bluetooth_address.is_some(),
    };
    for interface in interfaces {
        if let Ok(field) = hid_desc::battery_field_for_path(&interface.path) {
            detection.hid = Some((interface.path.clone(), field));
            break;
        }
    }
    detection
}

fn unavailable(note_ru: String, note_en: String) -> BatteryInfo {
    BatteryInfo {
        percent: None,
        charging: None,
        source: "unavailable".into(),
        status: "unavailable".into(),
        note_ru: Some(note_ru),
        note_en: Some(note_en),
        updated_at: now_ms(),
    }
}

fn found(
    percent: u8,
    charging: Option<bool>,
    source: &str,
    note_ru: String,
    note_en: String,
) -> BatteryInfo {
    let status = if charging == Some(true) {
        "charging"
    } else {
        "real"
    };
    BatteryInfo {
        percent: Some(percent.min(100)),
        charging,
        source: source.into(),
        status: status.into(),
        note_ru: Some(note_ru),
        note_en: Some(note_en),
        updated_at: now_ms(),
    }
}

fn vendor_probe(device: &DeviceView) -> Option<BatteryInfo> {
    if device.adapter_id == "redsquare" {
        return Some(match alumix::read_status(device) {
            Ok(status) => match status.battery {
                Some(percent) => found(
                    percent,
                    None,
                    "vendor",
                    format!("Заряд прочитан командой 0x8B Red Square Gsky: {percent}%."),
                    format!("Battery read with Red Square Gsky command 0x8B: {percent}%."),
                ),
                None => {
                    let raw = status
                        .battery_raw
                        .map(|value| format!(" (raw 0x{value:02X})"))
                        .unwrap_or_default();
                    unavailable(
                        format!("Red Square 0x8B не вернул процент заряда{raw}; значение не подменяется оценкой."),
                        format!("Red Square 0x8B returned no percentage{raw}; the value is not replaced by an estimate."),
                    )
                }
            },
            Err(error) => match error.split_once(" / ") {
                Some((ru, en)) => unavailable(ru.to_string(), en.to_string()),
                None => unavailable(
                    format!("Vendor-опрос Red Square не удался: {error}."),
                    format!("The Red Square vendor probe failed: {error}."),
                ),
            },
        });
    }
    if device.adapter_id == "ajazz" {
        return Some(match ajazz::read_status(device) {
            Ok(status) => match status.battery {
                Some(percent) => {
                    let push = status.source == "vendor-hid-push";
                    let charge_ru = if status.charging == Some(true) {
                        ", идёт зарядка"
                    } else {
                        ""
                    };
                    let charge_en = if status.charging == Some(true) {
                        ", charging"
                    } else {
                        ""
                    };
                    let frame_ru = if push {
                        "push-кадр 06 FA 03"
                    } else {
                        "кадр статуса 06 05"
                    };
                    let frame_en = if push {
                        "the 06 FA 03 push frame"
                    } else {
                        "the 06 05 status frame"
                    };
                    found(
                        percent,
                        status.charging,
                        "vendor",
                        format!("Заряд прочитан по NearLink vendor HID{charge_ru}. Источник: {frame_ru}."),
                        format!("Battery read over NearLink vendor HID{charge_en}. Source: {frame_en}."),
                    )
                }
                None => unavailable(
                    "NearLink ответил, но процент заряда в кадре отсутствует.".into(),
                    "NearLink answered, but the frame has no battery percentage.".into(),
                ),
            },
            Err(error) => match error.split_once(" / ") {
                Some((ru, en)) => unavailable(format!("NearLink: {ru}"), format!("NearLink: {en}")),
                None => unavailable(
                    format!("NearLink vendor-опроса не удалось: {error}."),
                    format!("The NearLink vendor probe failed: {error}."),
                ),
            },
        });
    }
    None
}

pub fn probe(device: &DeviceView) -> BatteryInfo {
    if let Some(result) = vendor_probe(device) {
        crate::store::log_line(&format!(
            "battery {}: source={} status={} percent={:?}",
            device.id, result.source, result.status, result.percent
        ));
        return result;
    }

    let mut attempts: Vec<String> = Vec::new();
    if let Some(address_text) = &device.bluetooth_address {
        if let Some(address) = ble::address_from_instance(address_text) {
            attempts.push("BLE GATT 0x180F/0x2A19".into());
            match ble::read_battery(address) {
                Ok(percent) => {
                    let result = found(
                        percent,
                        None,
                        "ble-gatt",
                        format!("Заряд прочитан из BLE Battery Service (0x180F/0x2A19): {percent}%."),
                        format!("Battery read from the BLE Battery Service (0x180F/0x2A19): {percent}%."),
                    );
                    crate::store::log_line(&format!(
                        "battery {}: ble-gatt percent={percent}",
                        device.id
                    ));
                    return result;
                }
                Err(error) => attempts.push(format!("BLE: {error}")),
            }
        }
    }

    let detection = detect(&device.interfaces, device.bluetooth_address.as_deref());
    if let Some((path, field)) = detection.hid {
        attempts.push(format!(
            "HID Battery Strength {} report {}",
            field.report_id, field.kind
        ));
        match hid_desc::read_field(&path, &field) {
            Ok(percent) => {
                let result = found(
                    percent,
                    None,
                    "hid-descriptor",
                    format!("Заряд прочитан из HID Battery Strength (0x06:0x20), report {}: {percent}%.", field.report_id),
                    format!("Battery read from HID Battery Strength (0x06:0x20), report {}: {percent}%.", field.report_id),
                );
                crate::store::log_line(&format!(
                    "battery {}: hid-descriptor percent={percent}",
                    device.id
                ));
                return result;
            }
            Err(error) => attempts.push(format!("HID: {error}")),
        }
    } else {
        attempts.push("HID Battery Strength не найден".into());
    }

    let (note_ru, note_en) = adapters::battery_note(&device.adapter_id, false);
    let tail_ru = if attempts.is_empty() {
        String::new()
    } else {
        format!(" Проверено: {}.", attempts.join("; "))
    };
    let tail_en = if attempts.is_empty() {
        String::new()
    } else {
        format!(" Tried: {}.", attempts.join("; "))
    };
    let result = unavailable(
        format!("{}{}", note_ru.unwrap_or_default(), tail_ru),
        format!("{}{}", note_en.unwrap_or_default(), tail_en),
    );
    crate::store::log_line(&format!(
        "battery {}: unavailable ({})",
        device.id,
        attempts.join("; ")
    ));
    result
}
