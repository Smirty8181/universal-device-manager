use std::collections::{BTreeMap, HashSet};

use hidapi::HidApi;
use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Devices::Properties::*;
use windows::Win32::Foundation::{FILETIME, HWND};

use crate::adapters;
use crate::battery;
use crate::model::*;
use crate::registry;

const GENERIC_NAMES: &[&str] = &[
    "usb input device",
    "usb audio device",
    "hid-compliant",
    "composite device",
    "композитное устройство",
    "usb-устройство ввода",
    "устройство ввода usb",
    "usb аудиоустройство",
    "аудиоустройство usb",
];

struct Node {
    instance_id: String,
    parent_id: Option<String>,
    container_id: Option<String>,
    class_name: String,
    friendly: Option<String>,
    manufacturer: Option<String>,
    service: Option<String>,
    version: Option<String>,
    provider: Option<String>,
    date: Option<String>,
    vid: Option<u16>,
    pid: Option<u16>,
    hardware_ids: Vec<String>,
    bluetooth: bool,
    bluetooth_address: Option<String>,
}

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

fn parse_vid_pid(text: &str) -> Option<(u16, u16)> {
    let upper = text.to_ascii_uppercase();
    let vid_pos = upper.find("VID_")? + 4;
    let pid_pos = upper.find("PID_")? + 4;
    let vid = u16::from_str_radix(upper.get(vid_pos..vid_pos + 4)?, 16).ok()?;
    let pid = u16::from_str_radix(upper.get(pid_pos..pid_pos + 4)?, 16).ok()?;
    Some((vid, pid))
}

fn guid_string(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 16 {
        return None;
    }
    let d1 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let d2 = u16::from_le_bytes([bytes[4], bytes[5]]);
    let d3 = u16::from_le_bytes([bytes[6], bytes[7]]);
    if d1 == 0 && d2 == 0 && d3 == 0 && bytes[8..16].iter().all(|b| *b == 0) {
        return None;
    }
    Some(format!(
        "{d1:08x}-{d2:04x}-{d3:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
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

unsafe fn prop_date(h: HDEVINFO, did: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<String> {
    let buf = prop_bytes(h, did, key)?;
    if buf.len() < 8 {
        return None;
    }
    let ft = FILETIME {
        dwLowDateTime: u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
        dwHighDateTime: u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
    };
    filetime_to_date(ft)
}

fn filetime_to_date(ft: FILETIME) -> Option<String> {
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
    let end = buf.iter().position(|w| *w == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

unsafe fn parent_id(devinst: u32) -> Option<String> {
    let mut parent = 0u32;
    if CM_Get_Parent(&mut parent, devinst, 0).0 != 0 {
        return None;
    }
    let mut buf = [0u16; 512];
    if CM_Get_Device_IDW(parent, &mut buf, 0).0 != 0 {
        return None;
    }
    let end = buf.iter().position(|w| *w == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

fn bluetooth_address(instance_id: &str) -> Option<String> {
    let upper = instance_id.to_ascii_uppercase();
    let mut search = upper.as_str();
    while let Some(position) = search.find("DEV_") {
        let start = position + 4;
        let candidate = search.get(start..start + 12)?;
        if candidate.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(candidate.to_string());
        }
        search = search.get(start..)?;
    }
    None
}

fn is_infrastructure(node: &Node) -> bool {
    let instance = node.instance_id.to_ascii_uppercase();
    if instance.starts_with("ROOT\\") || instance.starts_with("SWD\\") {
        return true;
    }
    let class = node.class_name.to_ascii_lowercase();
    if matches!(
        class.as_str(),
        "usb"
            | "system"
            | "softwaredevice"
            | "volume"
            | "diskdrive"
            | "cdrom"
            | "net"
            | "scsiadapter"
            | "hdc"
            | "ports"
            | "printer"
            | "printqueue"
            | "battery"
            | "firmware"
            | "securitydevices"
            | "smartcardreader"
            | "smartcard"
            | "display"
            | "monitor"
            | "processor"
    ) {
        return true;
    }
    let name = node
        .friendly
        .clone()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.contains("root hub")
        || name.contains("host controller")
        || name.contains("generic usb hub")
        || name.contains("usb hub")
        || name.contains("корневой usb-концентратор")
    {
        return true;
    }
    if class == "bluetooth" {
        let looks_like_hid = name.contains("hid")
            || name.contains("input")
            || name.contains("mouse")
            || name.contains("keyboard");
        if !looks_like_hid {
            return true;
        }
    }
    false
}

fn enumerate_nodes() -> Vec<Node> {
    let mut nodes = Vec::new();
    unsafe {
        let Ok(h) = SetupDiGetClassDevsW(
            None,
            PCWSTR::null(),
            HWND::default(),
            DIGCF_PRESENT | DIGCF_ALLCLASSES,
        ) else {
            return nodes;
        };
        let _guard = HDevGuard(h);
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
            let class_name = reg_string(h, &did, SPDRP_CLASS).unwrap_or_default();
            let hardware_ids = reg_list(h, &did, SPDRP_HARDWAREID);
            let ids_text = format!("{} {}", inst, hardware_ids.join(" "));
            let (vid, pid) = match parse_vid_pid(&ids_text) {
                Some((vid, pid)) => (Some(vid), Some(pid)),
                None => (None, None),
            };
            let friendly = prop_string(h, &did, &DEVPKEY_Device_FriendlyName)
                .or_else(|| reg_string(h, &did, SPDRP_DEVICEDESC));
            let upper = inst.to_ascii_uppercase();
            let bluetooth = upper.starts_with("BTHLE\\") || upper.starts_with("BTHENUM\\");
            let node = Node {
                container_id: prop_bytes(h, &did, &DEVPKEY_Device_ContainerId)
                    .and_then(|bytes| guid_string(&bytes)),
                bluetooth,
                bluetooth_address: if bluetooth {
                    bluetooth_address(&inst)
                } else {
                    None
                },
                parent_id: parent_id(did.DevInst),
                instance_id: inst,
                class_name,
                friendly,
                manufacturer: reg_string(h, &did, SPDRP_MFG),
                service: reg_string(h, &did, SPDRP_SERVICE),
                version: prop_string(h, &did, &DEVPKEY_Device_DriverVersion),
                provider: prop_string(h, &did, &DEVPKEY_Device_DriverProvider),
                date: prop_date(h, &did, &DEVPKEY_Device_DriverDate),
                vid,
                pid,
                hardware_ids,
            };
            if is_infrastructure(&node) {
                continue;
            }
            if node.vid.is_none() && node.pid.is_none() && !node.bluetooth {
                continue;
            }
            if node.friendly.is_none() && node.vid.is_none() && !node.bluetooth {
                continue;
            }
            nodes.push(node);
        }
    }
    nodes
}

struct HDevGuard(HDEVINFO);
impl Drop for HDevGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

struct HidNode {
    vendor_id: u16,
    product_id: u16,
    path: String,
    usage_page: u16,
    usage: u16,
    interface_number: Option<i32>,
    product: Option<String>,
    manufacturer: Option<String>,
    serial: Option<String>,
}

fn hid_nodes() -> Vec<HidNode> {
    let Ok(api) = HidApi::new() else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for info in api.device_list() {
        let path = info.path().to_string_lossy().into_owned();
        if !seen.insert(path.clone()) {
            continue;
        }
        out.push(HidNode {
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            path,
            usage_page: info.usage_page(),
            usage: info.usage(),
            interface_number: Some(info.interface_number()),
            product: info.product_string().map(|s| s.to_string()),
            manufacturer: info.manufacturer_string().map(|s| s.to_string()),
            serial: info.serial_number().map(|s| s.to_string()),
        });
    }
    out
}

fn is_generic_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    GENERIC_NAMES.iter().any(|g| lower.starts_with(g)) || lower.contains("hid-совместим")
}

fn pick_name(
    entry: Option<&registry::RegistryEntry>,
    nodes: &[&Node],
    hid: &[&HidNode],
    vid: Option<u16>,
    pid: Option<u16>,
) -> String {
    if let Some(entry) = entry {
        return format!("{} {}", entry.vendor, entry.model)
            .trim()
            .to_string();
    }
    for node in nodes {
        if let Some(name) = &node.friendly {
            if !is_generic_name(name) && !name.to_ascii_lowercase().contains("hid-") {
                return name.clone();
            }
        }
    }
    for h in hid {
        if let Some(name) = &h.product {
            if !name.trim().is_empty() {
                return name.trim().to_string();
            }
        }
    }
    for node in nodes {
        if let Some(name) = &node.friendly {
            return name.clone();
        }
    }
    match (vid, pid) {
        (Some(vid), Some(pid)) => format!("USB {:04X}:{:04X}", vid, pid),
        _ => "Bluetooth device".into(),
    }
}

fn serial_from_parent(parent: &Option<String>) -> Option<String> {
    let parent = parent.as_ref()?;
    let tail = parent.rsplit('\\').next()?.trim();
    if tail.is_empty() || tail.contains('&') || tail.chars().all(|c| c == '0') {
        None
    } else {
        Some(tail.to_string())
    }
}

fn infer_kind(
    entry: Option<&registry::RegistryEntry>,
    nodes: &[&Node],
    interfaces: &[HidInterface],
    name: &str,
) -> String {
    if let Some(entry) = entry {
        if !entry.category.trim().is_empty() {
            return entry.category.clone();
        }
    }
    let mut classes: Vec<String> = nodes
        .iter()
        .map(|n| n.class_name.to_ascii_lowercase())
        .collect();
    classes.sort();
    classes.dedup();
    let lower_name = name.to_ascii_lowercase();
    let text = |needles: &[&str]| needles.iter().any(|needle| lower_name.contains(needle));
    if classes.iter().any(|c| c == "keyboard") || text(&["keyboard", "клавиатур"]) {
        return "keyboard".into();
    }
    if classes.iter().any(|c| c == "mouse") || text(&["mouse", "мышь"]) {
        return "mouse".into();
    }
    if classes
        .iter()
        .any(|c| c == "xnacomposite" || c == "xboxcomposite")
        || text(&["gamepad", "controller", "joystick", "геймпад", "джойстик"])
    {
        return "gamepad".into();
    }
    if text(&["headset", "headphone", "наушник", "гарнитур"]) {
        return "headset".into();
    }
    if text(&["microphone", "микрофон"]) && !text(&["headset", "headphone"]) {
        return "microphone".into();
    }
    let has_audio = classes
        .iter()
        .any(|c| c == "media" || c == "audio" || c == "audioendpoint");
    if has_audio || text(&["speaker", "audio", "dac", "звук"]) {
        return "audio".into();
    }
    let keyboard_usage = interfaces
        .iter()
        .any(|i| i.usage_page == 0x01 && i.usage == 0x06);
    let mouse_usage = interfaces
        .iter()
        .any(|i| i.usage_page == 0x01 && i.usage == 0x02);
    if keyboard_usage {
        return "keyboard".into();
    }
    if mouse_usage {
        return "mouse".into();
    }
    if interfaces
        .iter()
        .any(|i| i.usage_page == 0x01 && matches!(i.usage, 0x04 | 0x05))
    {
        return "gamepad".into();
    }
    if classes.iter().any(|c| c == "hidclass") || !interfaces.is_empty() {
        return "hid".into();
    }
    "other".into()
}

fn connection_for(
    adapter_id: &str,
    pid: Option<u16>,
    nodes: &[&Node],
    product: Option<&str>,
) -> (String, String) {
    if nodes.iter().any(|n| n.bluetooth) {
        let le = nodes
            .iter()
            .any(|n| n.instance_id.to_ascii_uppercase().starts_with("BTHLE\\"));
        return (
            "bluetooth".into(),
            if le {
                "Bluetooth LE".into()
            } else {
                "Bluetooth".into()
            },
        );
    }
    if adapter_id == "redsquare" {
        return if pid == Some(0xE411) {
            ("wireless".into(), "2.4G (RSQ_20055_2.4G)".into())
        } else {
            ("usb".into(), "USB".into())
        };
    }
    if adapter_id == "ajazz" {
        let near_link = pid == Some(0x6426)
            || product
                .map(|p| p.to_ascii_lowercase().contains("nearlink"))
                .unwrap_or(false);
        return if near_link {
            ("wireless".into(), "NearLink (2.4G)".into())
        } else {
            ("usb".into(), "USB".into())
        };
    }
    if adapter_id == "ioaudio" {
        return ("wireless".into(), "USB-ресивер (Wireless)".into());
    }
    let mut text = product.unwrap_or("").to_ascii_lowercase();
    for node in nodes {
        text.push(' ');
        text.push_str(&node.instance_id.to_ascii_lowercase());
        if let Some(name) = &node.friendly {
            text.push(' ');
            text.push_str(&name.to_ascii_lowercase());
        }
    }
    let wireless = text.contains("2.4g")
        || text.contains("nearlink")
        || text.contains("wireless")
        || text.contains("receiver")
        || text.contains("ресивер")
        || text.contains("dongle");
    if wireless {
        let label = if text.contains("2.4g") {
            let raw = product.unwrap_or("2.4G").trim();
            format!("2.4G ({raw})")
        } else if text.contains("nearlink") {
            "NearLink (2.4G)".into()
        } else {
            "USB-ресивер (Wireless)".into()
        };
        ("wireless".into(), label)
    } else {
        ("usb".into(), "USB".into())
    }
}

fn cap_status(caps: &[Capability], id: &str) -> Option<String> {
    caps.iter()
        .find(|cap| cap.id == id)
        .map(|cap| cap.status.clone())
}

fn has_vendor_interface(interfaces: &[HidInterface]) -> bool {
    interfaces.iter().any(|i| i.usage_page >= 0xFF00)
}

fn add_runtime_caps(
    caps: &mut Vec<Capability>,
    entry: Option<&registry::RegistryEntry>,
    kind: &str,
    interfaces: &[HidInterface],
    detection: &battery::Detection,
    adapter_id: &str,
) {
    if !caps.iter().any(|c| c.id == "diagnostics") {
        caps.push(Capability::new(
            "diagnostics",
            "working",
            "Отчёты, HID-дескрипторы и пробные чтения доступны в разделе диагностики.",
            "Reports, HID descriptors and probe reads are available in diagnostics.",
        ));
    }
    if has_vendor_interface(interfaces) && !caps.iter().any(|c| c.id == "vendor_hid") {
        caps.push(Capability::new(
            "vendor_hid",
            "working",
            "Найден vendor HID-интерфейс (usage page ≥ 0xFF00); назначение команд не предполагается.",
            "A vendor HID interface (usage page ≥ 0xFF00) was found; command meanings are not assumed.",
        ));
    }
    let source_detected = detection.hid.is_some() || detection.ble;
    if let Some(cap) = caps.iter_mut().find(|c| c.id == "battery_level") {
        if source_detected && !matches!(cap.status.as_str(), "working" | "partial") {
            cap.status = "partial".into();
            cap.note_ru = format!(
                "{}{}",
                cap.note_ru, " Источник заряда обнаружен: доступен опрос."
            );
            cap.note_en = format!(
                "{}{}",
                cap.note_en, " A battery source was detected and can be probed."
            );
        }
    } else if source_detected {
        caps.push(Capability::new(
            "battery_level",
            "partial",
            "Обнаружен источник заряда (HID Battery Strength или BLE GATT); значение читается опросом.",
            "A battery source (HID Battery Strength or BLE GATT) was detected; the value is read by a probe.",
        ));
    }
    if kind == "keyboard" {
        if let Some(cap) = caps.iter_mut().find(|c| c.id == "lighting") {
            if adapter_id != "redsquare" {
                if has_vendor_interface(interfaces) {
                    cap.status = "research".into();
                    cap.note_ru = "Vendor HID найден, но протокол подсветки не проверен. Управление заблокировано до подтверждения.".into();
                    cap.note_en = "A vendor HID interface exists, but the lighting protocol is not verified. Controls stay locked until confirmed.".into();
                } else {
                    cap.status = "unsupported".into();
                    cap.note_ru = "В интерфейсах нет vendor HID, пригодного для подсветки; команды не отправляются.".into();
                    cap.note_en = "No vendor HID interface suitable for lighting was found; no commands are sent.".into();
                }
            }
        } else if adapter_id != "redsquare" {
            caps.push(if has_vendor_interface(interfaces) {
                Capability::new(
                    "lighting",
                    "research",
                    "Vendor HID найден, протокол подсветки не проверен; управление заблокировано.",
                    "Vendor HID found; the lighting protocol is not verified, so controls stay locked.",
                )
            } else {
                Capability::new(
                    "lighting",
                    "unsupported",
                    "Нет vendor HID-интерфейса для подсветки.",
                    "No vendor HID interface for lighting.",
                )
            });
        }
    }
    let _ = entry;
}

fn modules_for(
    adapter_id: &str,
    kind: &str,
    caps: &[Capability],
    detection: &battery::Detection,
    has_vendor: bool,
) -> Vec<AdapterModuleView> {
    let mut modules = Vec::new();
    modules.push(AdapterModuleView {
        id: "detection".into(),
        label: "Detection".into(),
        status: "working".into(),
        provider: Some("windows-pnp + hidapi".into()),
        note_ru: "PnP, контейнер устройства, HWID, VID/PID, HID-интерфейсы.".into(),
        note_en: "PnP, device container, HWID, VID/PID, HID interfaces.".into(),
    });
    let battery_status = cap_status(caps, "battery_level").unwrap_or_else(|| "research".into());
    let battery_provider = if adapter_id == "ajazz" {
        Some("vendor-nearlink")
    } else if adapter_id == "redsquare" {
        Some("vendor-gsky-0x8B")
    } else if detection.ble {
        Some("ble-gatt-0x180F")
    } else if detection.hid.is_some() {
        Some("hid-battery-strength")
    } else {
        None
    };
    modules.push(AdapterModuleView {
        id: "battery".into(),
        label: "Battery".into(),
        status: battery_status,
        provider: battery_provider.map(|s| s.to_string()),
        note_ru: "Отдельный модуль; RGB и настройки от него не зависят.".into(),
        note_en: "Independent module; RGB and settings do not depend on it.".into(),
    });
    let rgb_status = cap_status(caps, "lighting").unwrap_or_else(|| {
        if has_vendor {
            "research".into()
        } else {
            "unsupported".into()
        }
    });
    modules.push(AdapterModuleView {
        id: "rgb".into(),
        label: "RGB".into(),
        status: rgb_status,
        provider: if adapter_id == "redsquare" {
            Some("redsquare-gsky-feature7".into())
        } else {
            None
        },
        note_ru: if adapter_id == "redsquare" {
            "Запись только по кабелю; 2.4G-донгл не принимает feature report 7.".into()
        } else {
            "Протокол подсветки не подтверждён. Команды не отправляются.".into()
        },
        note_en: if adapter_id == "redsquare" {
            "Cable writes only; the 2.4G dongle rejects feature report 7.".into()
        } else {
            "Lighting protocol is not verified. No commands are sent.".into()
        },
    });
    if kind == "keyboard" {
        modules.push(AdapterModuleView {
            id: "keyboard".into(),
            label: "Keyboard".into(),
            status: if adapter_id == "redsquare" {
                "working"
            } else {
                "unsupported"
            }
            .into(),
            provider: if adapter_id == "redsquare" {
                Some("redsquare-gsky".into())
            } else {
                None
            },
            note_ru: if adapter_id == "redsquare" {
                "Подсветка, polling, latency, сон и debounce читаются/пишутся.".into()
            } else {
                "Проверенного протокола клавиатурных настроек нет.".into()
            },
            note_en: if adapter_id == "redsquare" {
                "Lighting, polling, latency, sleep and debounce are read/written.".into()
            } else {
                "There is no verified keyboard settings protocol.".into()
            },
        });
    }
    if kind == "mouse" {
        modules.push(AdapterModuleView {
            id: "mouse".into(),
            label: "Mouse".into(),
            status: if adapter_id == "ajazz" {
                "working"
            } else {
                "unsupported"
            }
            .into(),
            provider: if adapter_id == "ajazz" {
                Some("ajazz-nearlink".into())
            } else {
                None
            },
            note_ru: if adapter_id == "ajazz" {
                "DPI, polling и заряд читаются/пишутся с подтверждением.".into()
            } else {
                "Проверенного протокола DPI/polling нет.".into()
            },
            note_en: if adapter_id == "ajazz" {
                "DPI, polling and battery are read/written with confirmation.".into()
            } else {
                "There is no verified DPI/polling protocol.".into()
            },
        });
    }
    if matches!(kind, "headset" | "audio" | "microphone") {
        modules.push(AdapterModuleView {
            id: "audio".into(),
            label: "Audio".into(),
            status: "working".into(),
            provider: Some("windows-coreaudio".into()),
            note_ru: "Громкость и баланс через Core Audio; DSP вендора не документирован.".into(),
            note_en: "Volume and balance through Core Audio; vendor DSP is undocumented.".into(),
        });
    }
    if matches!(kind, "keyboard" | "mouse") {
        modules.push(AdapterModuleView {
            id: "buttons".into(),
            label: "Button mapping".into(),
            status: if matches!(adapter_id, "redsquare" | "ajazz") {
                "research"
            } else {
                "unsupported"
            }
            .into(),
            provider: None,
            note_ru: "Переназначение кнопок на уровне устройства не проверено.".into(),
            note_en: "Hardware-level button remapping is not verified.".into(),
        });
    }
    modules.push(AdapterModuleView {
        id: "firmware".into(),
        label: "Firmware".into(),
        status: "unsupported".into(),
        provider: None,
        note_ru:
            "Прошивка не выполняется: нет подтверждённого официального способа восстановления."
                .into(),
        note_en: "No firmware flashing: there is no confirmed official recovery path.".into(),
    });
    modules
}

struct Group {
    nodes: Vec<Node>,
    interfaces: Vec<HidInterface>,
    vid: Option<u16>,
    pid: Option<u16>,
    container_id: Option<String>,
    bluetooth_address: Option<String>,
    hardware_ids: Vec<String>,
}

fn group_key(node: &Node) -> String {
    if let Some(container) = &node.container_id {
        return format!("container:{container}");
    }
    let upper = node.instance_id.to_ascii_uppercase();
    if (upper.starts_with("HID\\") || upper.starts_with("BTHENUM\\")) && node.parent_id.is_some() {
        return format!("parent:{}", node.parent_id.clone().unwrap_or_default());
    }
    format!("instance:{upper}")
}

fn assign_hid_interfaces(groups: &mut [Group], hids: Vec<HidNode>) {
    let mut claimed: HashSet<String> = HashSet::new();
    for hid in hids {
        if claimed.contains(&hid.path) {
            continue;
        }
        let lower = hid.path.to_ascii_lowercase();
        let mut best: Option<(usize, i32)> = None;
        for (index, group) in groups.iter().enumerate() {
            let mut score = 0i32;
            if group.vid == Some(hid.vendor_id) && group.pid == Some(hid.product_id) {
                score += 3;
            }
            if let Some(product) = &hid.product {
                let product = product.to_ascii_lowercase();
                if group.nodes.iter().any(|node| {
                    node.friendly
                        .as_ref()
                        .map(|name| {
                            let name = name.to_ascii_lowercase();
                            !product.is_empty()
                                && (name.contains(&product) || product.contains(&name))
                        })
                        .unwrap_or(false)
                }) {
                    score += 2;
                }
            }
            for node in &group.nodes {
                let tail = node
                    .instance_id
                    .rsplit('\\')
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if tail.len() >= 8 && lower.contains(&tail) {
                    score += 8;
                }
                if let Some(address) = &node.bluetooth_address {
                    if lower.contains(&address.to_ascii_lowercase()) {
                        score += 6;
                    }
                }
            }
            if score > 0 && best.map(|(_, value)| score > value).unwrap_or(true) {
                best = Some((index, score));
            }
        }
        if let Some((index, _)) = best {
            let group = &mut groups[index];
            if group.interfaces.iter().any(|i| i.path == hid.path) {
                continue;
            }
            claimed.insert(hid.path.clone());
            group.interfaces.push(HidInterface {
                path: hid.path,
                usage_page: hid.usage_page,
                usage: hid.usage,
                interface_number: hid.interface_number,
                product: hid.product,
                manufacturer: hid.manufacturer,
                serial: hid.serial,
            });
        }
    }
}

fn group_devices(nodes: Vec<Node>, hids: Vec<HidNode>) -> Vec<DeviceView> {
    let mut buckets: BTreeMap<String, Vec<Node>> = BTreeMap::new();
    for node in nodes {
        buckets.entry(group_key(&node)).or_default().push(node);
    }
    let mut groups: Vec<Group> = Vec::new();
    for (_, nodes) in buckets {
        if nodes.is_empty() {
            continue;
        }
        let mut group = Group {
            vid: nodes.iter().find_map(|n| n.vid),
            pid: nodes.iter().find_map(|n| n.pid),
            container_id: nodes.iter().find_map(|n| n.container_id.clone()),
            bluetooth_address: nodes.iter().find_map(|n| n.bluetooth_address.clone()),
            hardware_ids: {
                let mut ids = Vec::new();
                for node in &nodes {
                    for id in &node.hardware_ids {
                        if !ids
                            .iter()
                            .any(|existing: &String| existing.eq_ignore_ascii_case(id))
                        {
                            ids.push(id.clone());
                        }
                    }
                }
                ids
            },
            interfaces: Vec::new(),
            nodes,
        };
        group.nodes.sort_by_key(|node| {
            !matches!(
                node.class_name.to_ascii_lowercase().as_str(),
                "keyboard" | "mouse" | "hidclass" | "media" | "xnacomposite" | "xboxcomposite"
            )
        });
        groups.push(group);
    }
    assign_hid_interfaces(&mut groups, hids);

    let now = now_ms();
    let mut devices = Vec::new();
    for group in groups {
        if group.vid.is_none() && group.interfaces.is_empty() && group.bluetooth_address.is_none() {
            continue;
        }
        let refs: Vec<&Node> = group.nodes.iter().collect();
        let group_hids: Vec<HidNode> = group
            .interfaces
            .iter()
            .map(|i| HidNode {
                vendor_id: group.vid.unwrap_or(0),
                product_id: group.pid.unwrap_or(0),
                path: i.path.clone(),
                usage_page: i.usage_page,
                usage: i.usage,
                interface_number: i.interface_number,
                product: i.product.clone(),
                manufacturer: i.manufacturer.clone(),
                serial: i.serial.clone(),
            })
            .collect();
        let hid_refs: Vec<&HidNode> = group_hids.iter().collect();
        let entry = match (group.vid, group.pid) {
            (Some(vid), Some(pid)) => registry::identify(vid, pid, &group.hardware_ids),
            _ => None,
        };
        let binding = match (group.vid, group.pid) {
            (Some(vid), Some(pid)) => registry::binding_for(vid, pid, &group.hardware_ids),
            _ => None,
        };
        let adapter_id = entry
            .as_ref()
            .map(|e| e.adapter_id.clone())
            .or_else(|| binding.as_ref().map(|b| b.adapter_id.clone()))
            .unwrap_or_else(|| "generic".into());
        let adapter_label = entry
            .as_ref()
            .map(|e| e.adapter_label.clone())
            .or_else(|| binding.as_ref().map(|b| b.adapter_label.clone()))
            .unwrap_or_else(|| "Generic HID Adapter".into());
        let product = group_hids.iter().find_map(|h| h.product.clone());
        let name = pick_name(entry.as_ref(), &refs, &hid_refs, group.vid, group.pid);
        let kind = infer_kind(entry.as_ref(), &refs, &group.interfaces, &name);
        let (connection, connection_label) =
            connection_for(&adapter_id, group.pid, &refs, product.as_deref());
        let vendor = entry
            .as_ref()
            .map(|e| e.vendor.clone())
            .or_else(|| group.nodes.iter().find_map(|n| n.manufacturer.clone()))
            .or_else(|| group_hids.iter().find_map(|h| h.manufacturer.clone()))
            .unwrap_or_else(|| "Неизвестный производитель".into());
        let model = entry
            .as_ref()
            .map(|e| e.model.clone())
            .or_else(|| group.nodes.iter().find_map(|n| n.friendly.clone()))
            .or_else(|| product.clone())
            .unwrap_or_default();
        let mut capabilities = match &entry {
            Some(entry) => entry.capabilities.clone(),
            None => match kind.as_str() {
                "headset" => adapters::headset_caps(),
                "audio" | "microphone" => adapters::audio_caps(),
                _ => adapters::generic_caps(),
            },
        };
        let detection = battery::detect(&group.interfaces, group.bluetooth_address.as_deref());
        add_runtime_caps(
            &mut capabilities,
            entry.as_ref(),
            &kind,
            &group.interfaces,
            &detection,
            &adapter_id,
        );
        let modules = modules_for(
            &adapter_id,
            &kind,
            &capabilities,
            &detection,
            has_vendor_interface(&group.interfaces),
        );
        let (note_ru, note_en) =
            adapters::battery_note(&adapter_id, detection.hid.is_some() || detection.ble);
        let battery = BatteryInfo {
            percent: None,
            charging: None,
            source: "unavailable".into(),
            status: "unknown".into(),
            note_ru,
            note_en,
            updated_at: now,
        };
        let driver_node = group
            .nodes
            .iter()
            .find(|n| n.version.is_some())
            .or_else(|| group.nodes.first());
        let driver = driver_node.map(|n| DriverInfo {
            version: n.version.clone(),
            provider: n.provider.clone(),
            date: n.date.clone(),
            service: n.service.clone(),
        });
        let parent_path = group
            .nodes
            .iter()
            .find(|n| {
                let id = n.instance_id.to_ascii_uppercase();
                (id.starts_with("USB\\VID_") && !id.contains("&MI_")) || id.starts_with("BTHLE\\")
            })
            .map(|n| n.instance_id.clone())
            .or_else(|| group.nodes.first().map(|n| n.instance_id.clone()));
        let serial = group_hids
            .iter()
            .find_map(|h| h.serial.clone())
            .or_else(|| {
                group
                    .nodes
                    .iter()
                    .find_map(|n| serial_from_parent(&Some(n.instance_id.clone())))
                    .or_else(|| {
                        serial_from_parent(&group.nodes.first().and_then(|n| n.parent_id.clone()))
                    })
            });
        let serial_clean = serial.clone().filter(|s| !s.trim().is_empty());
        let id_tail = serial_clean
            .clone()
            .or_else(|| group.container_id.clone())
            .or_else(|| group.nodes.iter().find_map(|n| n.bluetooth_address.clone()))
            .unwrap_or_else(|| {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                parent_path.hash(&mut hasher);
                format!("{:08x}", hasher.finish() as u32)
            });
        let prefix = if connection == "bluetooth" {
            "bt"
        } else {
            "usb"
        };
        let id = format!(
            "{}-{}-{}-{}",
            prefix,
            group
                .vid
                .map(|v| format!("{v:04x}"))
                .unwrap_or_else(|| "0000".into()),
            group
                .pid
                .map(|p| format!("{p:04x}"))
                .unwrap_or_else(|| "0000".into()),
            sanitize_id(&id_tail)
        );
        devices.push(DeviceView {
            id,
            kind,
            name,
            vendor,
            model,
            vid: group.vid,
            pid: group.pid,
            serial: serial_clean,
            connection,
            connection_label,
            firmware: None,
            battery: Some(battery),
            online: true,
            adapter_id,
            adapter_label,
            registry_id: entry.as_ref().map(|e| e.id.clone()),
            modules,
            container_id: group.container_id.clone(),
            capabilities,
            interfaces: group.interfaces,
            hardware_ids: group.hardware_ids.clone(),
            driver,
            physical_path: parent_path,
            bluetooth_address: group.bluetooth_address.clone(),
            detected_at: now,
            updated_at: now,
        });
    }
    let order = |kind: &str| match kind {
        "keyboard" => 0,
        "mouse" => 1,
        "headset" => 2,
        "audio" => 3,
        "gamepad" => 4,
        "hid" => 5,
        _ => 6,
    };
    devices.sort_by(|a, b| {
        order(&a.kind)
            .cmp(&order(&b.kind))
            .then_with(|| a.name.cmp(&b.name))
    });
    devices
}

fn sanitize_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(40).collect()
}

fn merge_transports(devices: Vec<DeviceView>) -> Vec<DeviceView> {
    let mut out: Vec<DeviceView> = Vec::new();
    for dev in devices {
        let existing = out.iter_mut().find(|d| {
            d.adapter_id != "generic"
                && d.adapter_id == dev.adapter_id
                && d.name == dev.name
                && matches!(
                    (d.connection.as_str(), dev.connection.as_str()),
                    ("usb", "wireless") | ("wireless", "usb")
                )
        });
        if let Some(base) = existing {
            base.connection_label = format!("{} · {}", base.connection_label, dev.connection_label);
            for iface in dev.interfaces {
                if !base.interfaces.iter().any(|i| i.path == iface.path) {
                    base.interfaces.push(iface);
                }
            }
            if base.battery.as_ref().and_then(|b| b.percent).is_none()
                && dev.battery.as_ref().and_then(|b| b.percent).is_some()
            {
                base.battery = dev.battery;
            }
            base.updated_at = dev.updated_at;
            continue;
        }
        out.push(dev);
    }
    out
}

pub fn enumerate_devices() -> Vec<DeviceView> {
    let nodes = enumerate_nodes();
    let hids = hid_nodes();
    merge_transports(group_devices(nodes, hids))
}

pub fn probe_battery(device: &DeviceView) -> BatteryInfo {
    crate::battery::probe(device)
}
