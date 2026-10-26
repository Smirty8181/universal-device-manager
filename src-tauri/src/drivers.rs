use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Devices::Properties::*;
use windows::Win32::Foundation::{FILETIME, HWND};

use crate::model::{now_ms, DriverEntry};

fn wide_string(bytes: &[u8]) -> String {
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let end = words.iter().position(|w| *w == 0).unwrap_or(words.len());
    String::from_utf16_lossy(&words[..end]).trim().to_string()
}

fn wide_list(bytes: &[u8]) -> Vec<String> {
    let words: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    words
        .split(|w| *w == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf16_lossy(part).trim().to_string())
        .collect()
}

pub(crate) fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|w| *w == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

fn parse_vid_pid(text: &str) -> (Option<u16>, Option<u16>) {
    let upper = text.to_ascii_uppercase();
    let parse = |key: &str| -> Option<u16> {
        let pos = upper.find(key)? + key.len();
        u16::from_str_radix(upper.get(pos..pos + 4)?, 16).ok()
    };
    (parse("VID_"), parse("PID_"))
}

unsafe fn reg_string(
    h: HDEVINFO,
    did: &SP_DEVINFO_DATA,
    prop: SETUP_DI_REGISTRY_PROPERTY,
) -> Option<String> {
    let mut size = 0u32;
    let _ = SetupDiGetDeviceRegistryPropertyW(h, did, prop, None, None, Some(&mut size));
    if size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    SetupDiGetDeviceRegistryPropertyW(h, did, prop, None, Some(&mut buf), None).ok()?;
    let value = wide_string(&buf);
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

unsafe fn reg_list(
    h: HDEVINFO,
    did: &SP_DEVINFO_DATA,
    prop: SETUP_DI_REGISTRY_PROPERTY,
) -> Vec<String> {
    let mut size = 0u32;
    let _ = SetupDiGetDeviceRegistryPropertyW(h, did, prop, None, None, Some(&mut size));
    if size == 0 {
        return Vec::new();
    }
    let mut buf = vec![0u8; size as usize];
    if SetupDiGetDeviceRegistryPropertyW(h, did, prop, None, Some(&mut buf), None).is_err() {
        return Vec::new();
    }
    wide_list(&buf)
}

unsafe fn prop_bytes(h: HDEVINFO, did: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<Vec<u8>> {
    let mut ty = DEVPROPTYPE::default();
    let mut size = 0u32;
    let _ = SetupDiGetDevicePropertyW(h, did, key, &mut ty, None, Some(&mut size), 0);
    if size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    SetupDiGetDevicePropertyW(h, did, key, &mut ty, Some(&mut buf), None, 0).ok()?;
    Some(buf)
}

unsafe fn prop_string(h: HDEVINFO, did: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<String> {
    let buf = prop_bytes(h, did, key)?;
    let value = wide_string(&buf);
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

unsafe fn prop_u32(h: HDEVINFO, did: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> u32 {
    let Some(buf) = prop_bytes(h, did, key) else {
        return 0;
    };
    if buf.len() < 4 {
        return 0;
    }
    u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]])
}

unsafe fn prop_date(h: HDEVINFO, did: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<String> {
    let buf = prop_bytes(h, did, key)?;
    if buf.len() < 8 {
        return None;
    }
    let ft = FILETIME {
        dwLowDateTime: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        dwHighDateTime: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
    };
    let ticks = ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64;
    if ticks < 116_444_736_000_000_000 {
        return None;
    }
    let secs = ((ticks - 116_444_736_000_000_000) / 10_000_000) as i64;
    chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.format("%d.%m.%Y").to_string())
}

unsafe fn instance_id(h: HDEVINFO, did: &SP_DEVINFO_DATA) -> Option<String> {
    let mut size = 0u32;
    let _ = SetupDiGetDeviceInstanceIdW(h, did, None, Some(&mut size));
    if size == 0 {
        return None;
    }
    let mut buf = vec![0u16; size as usize];
    SetupDiGetDeviceInstanceIdW(h, did, Some(&mut buf), None).ok()?;
    Some(wide_to_string(&buf))
}

fn problem_label(code: u32) -> Option<String> {
    if code == 0 {
        return None;
    }
    let label = match code {
        1 => "Устройство не настроено",
        3 => "Драйвер не загружен",
        10 => "Устройство не запускается",
        18 => "Требуется переустановка драйвера",
        19 | 20 => "Ошибка реестра или загрузки",
        21 => "Система перезагружается",
        22 => "Устройство отключено",
        24 => "Устройство отсутствует",
        28 => "Драйвер не установлен",
        31 => "Не удалось загрузить драйвер",
        32 => "Ошибка запуска службы драйвера",
        36..=39 => "Ошибка инициализации драйвера",
        43 => "Устройство остановлено из-за ошибки",
        _ => "Код проблемы",
    };
    Some(label.to_string())
}

pub(crate) fn signer_of_path(path: &str) -> String {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut info: SP_INF_SIGNER_INFO_V2_W = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SP_INF_SIGNER_INFO_V2_W>() as u32;
        if !SetupVerifyInfFileW(PCWSTR(wide.as_ptr()), None, &mut info).as_bool() {
            return "unsigned".into();
        }
        let signer = wide_to_string(&info.DigitalSigner);
        if signer.is_empty() {
            "unknown".into()
        } else if signer.to_ascii_lowercase().contains("microsoft") {
            "windows".into()
        } else {
            "publisher".into()
        }
    }
}

fn verify_inf(inf_name: Option<&str>) -> String {
    let Some(inf_name) = inf_name else {
        return "unknown".into();
    };
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let path = std::path::Path::new(&root).join("inf").join(inf_name);
    if !path.exists() {
        return "unknown".into();
    }
    signer_of_path(&path.to_string_lossy())
}

struct DriverFacts {
    instance_id: String,
    class_name: String,
    friendly: Option<String>,
    problem: u32,
    hardware_ids: Vec<String>,
    version: Option<String>,
    provider: Option<String>,
    date: Option<String>,
    service: Option<String>,
    inf: Option<String>,
}

fn build_entry(facts: DriverFacts) -> DriverEntry {
    let DriverFacts {
        instance_id,
        class_name,
        friendly,
        problem,
        hardware_ids,
        version,
        provider,
        date,
        service,
        inf,
    } = facts;
    let ids_text = format!("{} {}", instance_id, hardware_ids.join(" "));
    let (usb_vid, usb_pid) = parse_vid_pid(&ids_text);
    let name = friendly
        .or_else(|| hardware_ids.first().cloned())
        .unwrap_or_else(|| instance_id.clone());
    DriverEntry {
        signature: verify_inf(inf.as_deref()),
        instance_id,
        name,
        class_name,
        problem_code: problem,
        problem_label: problem_label(problem),
        driver_version: version,
        driver_provider: provider,
        driver_date: date,
        service,
        driver_inf: inf.clone(),
        hardware_ids,
        usb_vid,
        usb_pid,
    }
}

pub(crate) fn scan_internal(only: Option<&str>) -> Vec<DriverEntry> {
    let mut entries = Vec::new();
    unsafe {
        let Ok(h) = SetupDiGetClassDevsW(
            None,
            PCWSTR::null(),
            HWND::default(),
            DIGCF_PRESENT | DIGCF_ALLCLASSES,
        ) else {
            return entries;
        };
        let mut index = 0u32;
        loop {
            let mut did = SP_DEVINFO_DATA {
                cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiEnumDeviceInfo(h, index, &mut did).is_err() {
                break;
            }
            index += 1;
            let Some(inst) = instance_id(h, &did) else {
                continue;
            };
            if let Some(filter) = only {
                if !inst.eq_ignore_ascii_case(filter) {
                    continue;
                }
            }
            let class_name = reg_string(h, &did, SPDRP_CLASS).unwrap_or_default();
            let hardware_ids = reg_list(h, &did, SPDRP_HARDWAREID);
            let problem = prop_u32(h, &did, &DEVPKEY_Device_ProblemCode);
            let relevant = problem != 0
                || matches!(
                    class_name.as_str(),
                    "Keyboard" | "Mouse" | "HIDClass" | "MEDIA" | "AudioEndpoint" | "Bluetooth"
                )
                || hardware_ids
                    .iter()
                    .any(|id| id.to_ascii_uppercase().starts_with("USB\\VID_"));
            if only.is_none() && !relevant {
                continue;
            }
            let friendly = prop_string(h, &did, &DEVPKEY_Device_FriendlyName)
                .or_else(|| reg_string(h, &did, SPDRP_DEVICEDESC));
            let entry = build_entry(DriverFacts {
                instance_id: inst,
                class_name,
                friendly,
                problem,
                hardware_ids,
                version: prop_string(h, &did, &DEVPKEY_Device_DriverVersion),
                provider: prop_string(h, &did, &DEVPKEY_Device_DriverProvider),
                date: prop_date(h, &did, &DEVPKEY_Device_DriverDate),
                service: reg_string(h, &did, SPDRP_SERVICE),
                inf: prop_string(h, &did, &DEVPKEY_Device_DriverInfPath),
            });
            entries.push(entry);
        }
        let _ = SetupDiDestroyDeviceInfoList(h);
    }
    entries.sort_by(|a, b| {
        b.problem_code
            .cmp(&a.problem_code)
            .then_with(|| a.name.cmp(&b.name))
    });
    entries
}

pub fn scan() -> Vec<DriverEntry> {
    crate::store::log_line("driver scan started");
    scan_internal(None)
}

pub fn verify(instance: &str) -> Result<DriverEntry, String> {
    let mut found = scan_internal(Some(instance));
    if found.is_empty() {
        return Err(format!("device not found: {instance}"));
    }
    let entry = found.remove(0);
    crate::store::log_line(&format!(
        "driver verify: {} → {} (problem {})",
        entry.name, entry.signature, entry.problem_code
    ));
    Ok(entry)
}

pub fn record_history(state: &crate::store::AppState, entry: &DriverEntry) -> Result<(), String> {
    let result = if entry.problem_code == 0 {
        format!(
            "Подпись: {} · версия {}",
            entry.signature,
            entry.driver_version.as_deref().unwrap_or("—")
        )
    } else {
        format!(
            "Проблема: {} · подпись {}",
            entry.problem_label.as_deref().unwrap_or("—"),
            entry.signature
        )
    };
    {
        let mut history = state.driver_history.lock().unwrap();
        history.insert(
            0,
            crate::model::DriverHistoryEntry {
                at: now_ms(),
                instance_id: entry.instance_id.clone(),
                name: entry.name.clone(),
                result,
            },
        );
        history.truncate(100);
    }
    state.save_driver_history()
}
