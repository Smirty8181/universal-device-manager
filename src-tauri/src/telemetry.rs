use std::ffi::CString;
use std::time::{Duration, Instant};

use hidapi::HidApi;
use serde::{Deserialize, Serialize};

use crate::hid_desc::HidField;
use crate::model::{now_ms, BatteryInfo, DeviceView};
use crate::{ajazz, alumix, battery, hid_desc, hid_lab, input};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryValue {
    pub key: String,
    pub label_ru: String,
    pub label_en: String,
    pub value: Option<String>,
    pub source: String,
    pub method: String,
    pub confidence: String,
    pub at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryProtocol {
    pub path: String,
    pub usage_page: u16,
    pub usage: u16,
    pub interface_number: Option<i32>,
    pub product: Option<String>,
    pub vendor_specific: bool,
    pub input_reports: Vec<u8>,
    pub output_reports: Vec<u8>,
    pub feature_reports: Vec<u8>,
    pub field_count: usize,
    pub battery_field: Option<String>,
    pub descriptor_ok: bool,
    pub note_ru: String,
    pub note_en: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReceiverInfo {
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub name: String,
    pub transport: String,
    pub adapter: Option<String>,
    pub protocol_known: bool,
    pub battery_channel: String,
    pub battery_state: String,
    pub battery_percent: Option<u8>,
    pub interfaces: usize,
    pub vendor_interfaces: usize,
    pub note_ru: String,
    pub note_en: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryNote {
    pub level: String,
    pub ru: String,
    pub en: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryReport {
    pub device_id: String,
    pub device_name: String,
    pub kind: String,
    pub vendor: String,
    pub model: String,
    pub connection: String,
    pub connection_label: String,
    pub battery: BatteryInfo,
    pub battery_state: String,
    pub scanned_at: u64,
    pub values: Vec<TelemetryValue>,
    pub protocols: Vec<TelemetryProtocol>,
    pub receiver: Option<ReceiverInfo>,
    pub notes: Vec<TelemetryNote>,
    pub source: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeatureProbe {
    pub path: String,
    pub report_id: u8,
    pub length: usize,
    pub data: String,
    pub error: Option<String>,
    pub at: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReportSample {
    pub at: u64,
    pub report_id: u8,
    pub hex: String,
    pub repeats: u32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub path: String,
    pub duration_ms: u64,
    pub samples: Vec<ReportSample>,
    pub error: Option<String>,
}

fn value(
    key: &str,
    label_ru: &str,
    label_en: &str,
    data: Option<String>,
    source: &str,
    method: &str,
    confidence: &str,
) -> TelemetryValue {
    TelemetryValue {
        key: key.into(),
        label_ru: label_ru.into(),
        label_en: label_en.into(),
        value: data,
        source: source.into(),
        method: method.into(),
        confidence: confidence.into(),
        at: now_ms(),
    }
}

fn battery_state(device: &DeviceView, battery: &BatteryInfo) -> String {
    match battery.status.as_str() {
        "real" => "real".into(),
        "charging" => {
            if battery.percent == Some(100) {
                "full".into()
            } else {
                "charging".into()
            }
        }
        "estimated" => "estimated".into(),
        "disconnected" => "disconnected".into(),
        "unavailable" => {
            let receiver_only = device.connection == "wireless";
            if receiver_only && matches!(device.adapter_id.as_str(), "redsquare" | "ioaudio") {
                "unsupported".into()
            } else {
                "unavailable".into()
            }
        }
        _ => "unknown".into(),
    }
}

fn battery_method(source: &str) -> &'static str {
    match source {
        "vendor" => "vendor HID feature report",
        "ble-gatt" => "BLE GATT 0x180F/0x2A19",
        "hid-descriptor" => "HID Battery Strength 0x06:0x20",
        _ => "no readable battery channel",
    }
}

fn report_ids(fields: &[HidField], kind: &str) -> Vec<u8> {
    let mut ids: Vec<u8> = fields
        .iter()
        .filter(|field| field.kind == kind)
        .map(|field| field.report_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn protocol_for(interface: &crate::model::HidInterface) -> TelemetryProtocol {
    match hid_desc::caps_for_path(&interface.path) {
        Ok(caps) => {
            let battery = hid_desc::battery_field(&caps).map(|field| {
                format!(
                    "{} report {} · {} bit · logical 0..{}",
                    field.kind, field.report_id, field.bit_size, field.logical_max
                )
            });
            let vendor_specific = interface.usage_page >= 0xFF00;
            TelemetryProtocol {
                path: interface.path.clone(),
                usage_page: interface.usage_page,
                usage: interface.usage,
                interface_number: interface.interface_number,
                product: interface.product.clone(),
                vendor_specific,
                input_reports: report_ids(&caps.fields, "input"),
                output_reports: report_ids(&caps.fields, "output"),
                feature_reports: report_ids(&caps.fields, "feature"),
                field_count: caps.fields.len(),
                battery_field: battery,
                descriptor_ok: true,
                note_ru: if vendor_specific {
                    "Vendor HID: назначение полей не документировано. Разрешено только чтение."
                        .into()
                } else {
                    "Стандартная HID-коллекция; состав полей прочитан из дескриптора.".into()
                },
                note_en: if vendor_specific {
                    "Vendor HID: field meanings are undocumented. Only reading is allowed.".into()
                } else {
                    "Standard HID collection; its fields were read from the descriptor.".into()
                },
            }
        }
        Err(error) => TelemetryProtocol {
            path: interface.path.clone(),
            usage_page: interface.usage_page,
            usage: interface.usage,
            interface_number: interface.interface_number,
            product: interface.product.clone(),
            vendor_specific: interface.usage_page >= 0xFF00,
            input_reports: Vec::new(),
            output_reports: Vec::new(),
            feature_reports: Vec::new(),
            field_count: 0,
            battery_field: None,
            descriptor_ok: false,
            note_ru: format!("Дескриптор недоступен: {error}."),
            note_en: format!("Descriptor unavailable: {error}."),
        },
    }
}

fn receiver_for(device: &DeviceView, battery: &BatteryInfo) -> Option<ReceiverInfo> {
    let label = device.connection_label.to_ascii_lowercase();
    let wireless = device.connection == "wireless"
        || label.contains("2.4")
        || label.contains("receiver")
        || label.contains("dongle")
        || label.contains("ресивер");
    if !wireless {
        return None;
    }
    let vendor_interfaces = device
        .interfaces
        .iter()
        .filter(|i| i.usage_page >= 0xFF00)
        .count();
    let adapter = match device.adapter_id.as_str() {
        "redsquare" => Some("Red Square Gsky · feature report 7 / cmd 0x8B".to_string()),
        "ajazz" => Some("AJAZZ NearLink HID · 0xFFB1/0xFFB2/0xFFB3".to_string()),
        "ioaudio" => Some("C-Media HID · 0xFF22/0xFF90".to_string()),
        _ => None,
    };
    let protocol_known = adapter.is_some();
    let state = battery_state(device, battery);
    let battery_channel = match state.as_str() {
        "real" => "real",
        "charging" | "full" => "charging",
        "unsupported" => "unsupported",
        _ => "unknown",
    }
    .to_string();
    let (note_ru, note_en) = match device.adapter_id.as_str() {
        "redsquare" => (
            "Протокол ресивера известен: заряд читается командой 0x8B. Запись подсветки возможна только по кабелю.".to_string(),
            "The receiver protocol is known: battery is read with command 0x8B. Lighting writes work over cable only.".to_string(),
        ),
        "ajazz" => (
            "Протокол NearLink известен: заряд и DPI читаются через vendor HID.".to_string(),
            "The NearLink protocol is known: battery and DPI are read over vendor HID.".to_string(),
        ),
        "ioaudio" => (
            "Ресивер C-Media не имеет батарейного поля в 0xFF22/0xFF90. Заряд возможен только по Bluetooth Battery Service.".to_string(),
            "The C-Media receiver has no battery field in 0xFF22/0xFF90. Charge is only available over the Bluetooth Battery Service.".to_string(),
        ),
        _ => (
            "Windows видит только USB-ресивер; протокол не определён. Опрос ограничен чтением HID Battery Strength и feature-отчётов; запись не выполняется.".to_string(),
            "Windows only exposes the USB receiver and the protocol is unknown. Probing is limited to reading HID Battery Strength and feature reports; nothing is written.".to_string(),
        ),
    };
    Some(ReceiverInfo {
        vid: device.vid,
        pid: device.pid,
        name: device.name.clone(),
        transport: device.connection_label.clone(),
        adapter,
        protocol_known,
        battery_channel,
        battery_state: state,
        battery_percent: battery.percent,
        interfaces: device.interfaces.len(),
        vendor_interfaces,
        note_ru,
        note_en,
    })
}

fn redsquare_values(
    device: &DeviceView,
    values: &mut Vec<TelemetryValue>,
    notes: &mut Vec<TelemetryNote>,
) {
    match alumix::read_status(device) {
        Ok(status) => {
            if let Some(firmware) = status.firmware.clone() {
                values.push(value(
                    "firmware",
                    "Прошивка",
                    "Firmware",
                    Some(firmware),
                    "redsquare-gsky",
                    "feature report 7 · cmd 0x88",
                    "verified",
                ));
            }
            if let Some(polling) = status.polling_hz {
                values.push(value(
                    "polling",
                    "Polling rate",
                    "Polling rate",
                    Some(format!("{polling} Hz")),
                    "redsquare-gsky",
                    "feature report 7 status",
                    "verified",
                ));
            }
            values.push(value(
                "lighting",
                "Подсветка",
                "Lighting",
                Some(format!(
                    "эффект #{}, яркость {}, скорость {}",
                    status.effect_id, status.brightness, status.speed
                )),
                "redsquare-gsky",
                "feature report 7 · cmd 0x02 state",
                "verified",
            ));
            if let Some(raw) = status.battery_raw {
                values.push(value(
                    "battery_raw",
                    "Сырое значение заряда",
                    "Raw battery byte",
                    Some(format!("0x{raw:02X}")),
                    "redsquare-gsky",
                    "feature report 7 · cmd 0x8B",
                    "verified",
                ));
            }
        }
        Err(error) => notes.push(TelemetryNote {
            level: "warn".into(),
            ru: format!(
                "Vendor-опрос Red Square не удался ({error}). Значения заряда и подсветки остаются прежними."
            ),
            en: format!(
                "The Red Square vendor probe failed ({error}). Battery and lighting values are left unchanged."
            ),
        }),
    }
}

fn ajazz_values(
    device: &DeviceView,
    values: &mut Vec<TelemetryValue>,
    notes: &mut Vec<TelemetryNote>,
) {
    match ajazz::read_status(device) {
        Ok(status) => {
            if !status.dpi.is_empty() {
                values.push(value(
                    "dpi",
                    "DPI-этапы",
                    "DPI stages",
                    Some(
                        status
                            .dpi
                            .iter()
                            .map(|stage| stage.to_string())
                            .collect::<Vec<_>>()
                            .join(" / "),
                    ),
                    "ajazz-nearlink",
                    "vendor HID status frame 06 05",
                    "verified",
                ));
            }
            if let Some(stage) = status.active_stage {
                values.push(value(
                    "dpi_stage",
                    "Активный DPI-этап",
                    "Active DPI stage",
                    Some(format!("{}", stage + 1)),
                    "ajazz-nearlink",
                    "vendor HID status frame 06 05",
                    "verified",
                ));
            }
            if let Some(polling) = status.polling_hz {
                values.push(value(
                    "polling",
                    "Polling rate",
                    "Polling rate",
                    Some(format!("{polling} Hz")),
                    "ajazz-nearlink",
                    "vendor HID status frame 06 05",
                    "verified",
                ));
            }
        }
        Err(error) => notes.push(TelemetryNote {
            level: "warn".into(),
            ru: format!("NearLink-опрос не удался: {error}. Заряд и DPI не изменены."),
            en: format!("The NearLink probe failed: {error}. Battery and DPI are left unchanged."),
        }),
    }
}

pub fn scan(device: &DeviceView) -> TelemetryReport {
    let at = now_ms();
    let battery = battery::probe(device);
    let state = battery_state(device, &battery);
    let mut values = Vec::new();
    let mut notes = Vec::new();

    values.push(value(
        "vendor",
        "Производитель",
        "Manufacturer",
        Some(device.vendor.clone()),
        "windows-pnp",
        "SetupAPI / device container",
        "verified",
    ));
    values.push(value(
        "model",
        "Модель",
        "Model",
        Some(device.model.clone()),
        if device.registry_id.is_some() {
            "local-db"
        } else {
            "windows-pnp"
        },
        "VID/PID + HWID match",
        if device.registry_id.is_some() {
            "verified"
        } else {
            "supported"
        },
    ));
    values.push(value(
        "vid_pid",
        "VID / PID",
        "VID / PID",
        match (device.vid, device.pid) {
            (Some(vid), Some(pid)) => Some(format!("{vid:04X}:{pid:04X}")),
            _ => None,
        },
        "windows-pnp",
        "USB hardware ID",
        "verified",
    ));
    let hardware_ids = if device.hardware_ids.is_empty() {
        None
    } else {
        Some(device.hardware_ids.join(" · "))
    };
    values.push(value(
        "hardware_id",
        "Hardware ID",
        "Hardware IDs",
        hardware_ids.clone(),
        "windows-pnp",
        "SetupAPI SPDRP_HARDWAREID",
        if hardware_ids.is_some() {
            "verified"
        } else {
            "unknown"
        },
    ));
    let serial = device.serial.clone().filter(|s| !s.trim().is_empty());
    values.push(value(
        "serial",
        "Серийный номер",
        "Serial number",
        serial.clone(),
        "windows-pnp",
        "device instance / HID serial",
        if serial.is_some() {
            "verified"
        } else {
            "unknown"
        },
    ));
    values.push(value(
        "connection",
        "Подключение",
        "Connection",
        Some(device.connection_label.clone()),
        "windows-pnp",
        "PnP tree + transport heuristics",
        "supported",
    ));
    values.push(value(
        "driver",
        "Драйвер",
        "Driver",
        device.driver.as_ref().and_then(|d| {
            let mut parts = Vec::new();
            if let Some(version) = &d.version {
                parts.push(version.clone());
            }
            if let Some(provider) = &d.provider {
                parts.push(provider.clone());
            }
            if let Some(date) = &d.date {
                parts.push(date.clone());
            }
            if parts.is_empty() {
                None
            } else {
                Some(parts.join(" · "))
            }
        }),
        "windows-pnp",
        "DEVPKEY_Device_Driver*",
        "supported",
    ));
    values.push(value(
        "path",
        "Путь устройства",
        "Device path",
        device.physical_path.clone(),
        "windows-pnp",
        "CM_Get_Device_ID",
        if device.physical_path.is_some() {
            "verified"
        } else {
            "unknown"
        },
    ));

    values.push(value(
        "battery",
        "Заряд",
        "Battery",
        battery.percent.map(|percent| format!("{percent}%")),
        if battery.source.is_empty() {
            "unavailable"
        } else {
            &battery.source
        },
        battery_method(&battery.source),
        match state.as_str() {
            "real" | "charging" | "full" => "verified",
            "estimated" => "estimated",
            "unsupported" => "unsupported",
            _ => "unknown",
        },
    ));
    values.push(value(
        "battery_state",
        "Состояние заряда",
        "Battery state",
        Some(state.clone()),
        "battery-engine",
        "combined probe result",
        match state.as_str() {
            "real" | "charging" | "full" => "verified",
            "estimated" => "estimated",
            "unsupported" => "unsupported",
            _ => "unknown",
        },
    ));
    values.push(value(
        "charging",
        "Зарядка",
        "Charging",
        Some(
            match battery.charging {
                Some(true) => "да",
                Some(false) => "нет",
                None => "неизвестно",
            }
            .to_string(),
        ),
        if battery.source.is_empty() {
            "unavailable"
        } else {
            &battery.source
        },
        battery_method(&battery.source),
        if battery.charging.is_some() {
            "verified"
        } else {
            "unknown"
        },
    ));

    if device.adapter_id == "redsquare" {
        redsquare_values(device, &mut values, &mut notes);
    } else if device.adapter_id == "ajazz" {
        ajazz_values(device, &mut values, &mut notes);
    } else if device.adapter_id == "ioaudio" {
        notes.push(TelemetryNote {
            level: "info".into(),
            ru: "Аудиопоток и громкость доступны через Core Audio в Audio Studio; встроенный DSP вендора не документирован.".into(),
            en: "Audio playback and volume are available through Core Audio in Audio Studio; the vendor DSP is undocumented.".into(),
        });
    }

    let measured = input::stats(1000).into_iter().find(|stat| {
        stat.vid == device.vid.map(u32::from) && stat.pid == device.pid.map(u32::from)
    });
    if let Some(stat) = measured {
        values.push(value(
            "report_rate",
            "Частота отчётов (замер)",
            "Report rate (measured)",
            Some(format!("{:.0} /с · {} отчётов", stat.rate, stat.reports)),
            "windows-raw-input",
            "1 s observation window",
            "experimental",
        ));
    }

    let working_caps: Vec<String> = device
        .capabilities
        .iter()
        .filter(|cap| cap.status == "working")
        .map(|cap| cap.id.clone())
        .collect();
    values.push(value(
        "features",
        "Подтверждённые функции",
        "Verified features",
        if working_caps.is_empty() {
            None
        } else {
            Some(working_caps.join(", "))
        },
        "capability-engine",
        "local registry + runtime checks",
        "supported",
    ));
    values.push(value(
        "temperature",
        "Температура",
        "Temperature",
        None,
        "hid-descriptor",
        "no temperature field found",
        "unknown",
    ));
    values.push(value(
        "voltage",
        "Напряжение батареи",
        "Battery voltage",
        None,
        "hid-descriptor",
        "no voltage field found",
        "unknown",
    ));
    values.push(value(
        "signal",
        "Уровень сигнала",
        "Signal level",
        None,
        "windows-pnp",
        "receiver does not expose RSSI",
        "unknown",
    ));

    let protocols: Vec<TelemetryProtocol> = device.interfaces.iter().map(protocol_for).collect();
    let vendor_count = protocols.iter().filter(|p| p.vendor_specific).count();
    if vendor_count > 0 && device.registry_id.is_none() {
        notes.push(TelemetryNote {
            level: "info".into(),
            ru: format!(
                "Найдено vendor HID-интерфейсов: {vendor_count}. Команды не отправляются; доступно только чтение дескрипторов и отчётов."
            ),
            en: format!(
                "{vendor_count} vendor HID interface(s) found. No commands are sent; only descriptors and reports can be read."
            ),
        });
    }
    if device.interfaces.is_empty() {
        notes.push(TelemetryNote {
            level: "warn".into(),
            ru: "У устройства нет доступных HID-интерфейсов; телеметрия ограничена PnP и аудио.".into(),
            en: "The device has no accessible HID interfaces; telemetry is limited to PnP and audio.".into(),
        });
    }
    if state == "unsupported" {
        notes.push(TelemetryNote {
            level: "info".into(),
            ru: "Заряд не поддерживается текущим транспортом; проценты не подставляются и не оцениваются.".into(),
            en: "Battery is not supported over the current transport; no percentages are invented or estimated.".into(),
        });
    }
    let receiver = receiver_for(device, &battery);
    if receiver.as_ref().map(|r| r.protocol_known).unwrap_or(false)
        && device.connection == "wireless"
    {
        notes.push(TelemetryNote {
            level: "info".into(),
            ru: "За одним USB-ресивером Windows не разделяет несколько физических устройств; идентификация ограничена ресивером.".into(),
            en: "Windows does not separate multiple physical devices behind one USB receiver; identification is limited to the receiver.".into(),
        });
    }
    crate::store::log_line(&format!(
        "telemetry scan {}: values={} interfaces={} battery={} source={}",
        device.id,
        values.len(),
        protocols.len(),
        state,
        battery.source
    ));
    TelemetryReport {
        device_id: device.id.clone(),
        device_name: device.name.clone(),
        kind: device.kind.clone(),
        vendor: device.vendor.clone(),
        model: device.model.clone(),
        connection: device.connection.clone(),
        connection_label: device.connection_label.clone(),
        battery,
        battery_state: state,
        scanned_at: at,
        values,
        protocols,
        receiver,
        notes,
        source: "live".into(),
    }
}

fn report_length(caps: &hid_desc::HidCapsInfo, kind: &str, report_id: u8) -> usize {
    let total = match kind {
        "feature" => caps.feature_len,
        "output" => caps.output_len,
        _ => caps.input_len,
    };
    if total == 0 {
        return 1;
    }
    (total as usize)
        .saturating_sub(if report_id == 0 { 0 } else { 1 })
        .max(1)
}

pub fn feature_sweep(device: &DeviceView, path: &str) -> Result<Vec<FeatureProbe>, String> {
    if !device
        .interfaces
        .iter()
        .any(|interface| interface.path == path)
    {
        return Err("interface path does not belong to the selected device".into());
    }
    let caps = hid_desc::caps_for_path(path)?;
    if caps.feature_len == 0 {
        return Ok(Vec::new());
    }
    let mut ids: Vec<u8> = caps
        .fields
        .iter()
        .filter(|field| field.kind == "feature")
        .map(|field| field.report_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        ids.push(0);
    }
    let mut probes = Vec::new();
    for id in ids.into_iter().take(4) {
        let length = report_length(&caps, "feature", id);
        let result = hid_lab::read_feature(device, path, id, length)?;
        probes.push(FeatureProbe {
            path: path.to_string(),
            report_id: id,
            length,
            data: result.data,
            error: result.error,
            at: now_ms(),
        });
    }
    crate::store::log_line(&format!(
        "telemetry feature sweep: device={} path={} probes={}",
        device.id,
        path,
        probes.len()
    ));
    Ok(probes)
}

pub fn capture(device: &DeviceView, path: &str, duration_ms: u64) -> Result<CaptureResult, String> {
    if !device
        .interfaces
        .iter()
        .any(|interface| interface.path == path)
    {
        return Err("interface path does not belong to the selected device".into());
    }
    let api = HidApi::new().map_err(|e| e.to_string())?;
    let cpath = CString::new(path).map_err(|_| "invalid interface path".to_string())?;
    let handle = api.open_path(&cpath).map_err(|e| e.to_string())?;
    let _ = handle.set_blocking_mode(false);
    let started = Instant::now();
    let window = Duration::from_millis(duration_ms.clamp(300, 3000));
    let mut buffer = [0u8; 512];
    let mut samples: Vec<ReportSample> = Vec::new();
    while started.elapsed() < window && samples.len() < 48 {
        match handle.read_timeout(&mut buffer, 20) {
            Ok(read) if read > 0 => {
                let raw = &buffer[..read];
                let report_id = if raw[0] == 0 { 0 } else { raw[0] };
                let hex = hid_desc::hex_bytes(raw);
                if let Some(last) = samples.last_mut() {
                    if last.hex == hex {
                        last.repeats = last.repeats.saturating_add(1);
                        continue;
                    }
                }
                samples.push(ReportSample {
                    at: now_ms(),
                    report_id,
                    hex,
                    repeats: 1,
                });
            }
            _ => std::thread::sleep(Duration::from_millis(4)),
        }
    }
    let _ = handle.set_blocking_mode(true);
    crate::store::log_line(&format!(
        "telemetry capture: device={} path={} samples={} window={}ms",
        device.id,
        path,
        samples.len(),
        window.as_millis()
    ));
    Ok(CaptureResult {
        path: path.to_string(),
        duration_ms: window.as_millis() as u64,
        samples,
        error: None,
    })
}
