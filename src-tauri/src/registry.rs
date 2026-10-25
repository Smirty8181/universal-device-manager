use std::sync::{OnceLock, RwLock};

use serde::Serialize;

use crate::adapters;
use crate::model::Capability;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySource {
    pub label: String,
    pub url: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEntry {
    pub id: String,
    pub vendor: String,
    pub model: String,
    pub category: String,
    pub adapter_id: String,
    pub adapter_label: String,
    pub vid_pids: Vec<String>,
    pub hardware_ids: Vec<String>,
    pub hardware_revision: Option<String>,
    pub firmware: Vec<String>,
    pub connections: Vec<String>,
    pub protocol_version: Option<String>,
    pub protocol_ru: String,
    pub protocol_en: String,
    pub verification: String,
    pub last_checked: String,
    pub official_software: Option<String>,
    pub capabilities: Vec<Capability>,
    pub sources: Vec<RegistrySource>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AdapterBinding {
    pub id: String,
    pub adapter_id: String,
    pub adapter_label: String,
    pub vid: u16,
    pub pid: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_id: Option<String>,
    pub priority: i32,
    pub verification: String,
    pub source: String,
}

#[derive(Default)]
struct DeviceCatalog {
    entries: Vec<RegistryEntry>,
    bindings: Vec<AdapterBinding>,
}

static CATALOG: OnceLock<RwLock<DeviceCatalog>> = OnceLock::new();

fn catalog() -> &'static RwLock<DeviceCatalog> {
    CATALOG.get_or_init(|| RwLock::new(DeviceCatalog::default()))
}

pub fn set_catalog(entries: Vec<RegistryEntry>, bindings: Vec<AdapterBinding>) {
    let mut guard = catalog().write().unwrap();
    guard.entries = entries;
    guard.bindings = bindings;
}

pub fn catalog_snapshot() -> (Vec<RegistryEntry>, Vec<AdapterBinding>) {
    let guard = catalog().read().unwrap();
    (guard.entries.clone(), guard.bindings.clone())
}

fn norm(value: &str) -> String {
    value.to_ascii_uppercase().replace(' ', "")
}

pub fn parse_vid_pid(text: &str) -> Option<(u16, u16)> {
    let upper = text.to_ascii_uppercase();
    let start = upper.find("VID_").map(|i| i + 4).or_else(|| {
        let colon = upper.find(':')?;
        Some(colon.saturating_sub(4))
    })?;
    let vid = u16::from_str_radix(upper.get(start..start + 4)?, 16).ok()?;
    let pid_start = upper
        .find("PID_")
        .map(|i| i + 4)
        .or_else(|| upper.find(':').map(|i| i + 1))?;
    let pid = u16::from_str_radix(upper.get(pid_start..pid_start + 4)?, 16).ok()?;
    Some((vid, pid))
}

fn binding_matches(binding: &AdapterBinding, vid: u16, pid: u16, hardware_ids: &[String]) -> bool {
    if binding.vid != vid || binding.pid != pid {
        return false;
    }
    match &binding.hardware_id {
        Some(pattern) => {
            let pattern = norm(pattern);
            hardware_ids.iter().any(|id| norm(id).starts_with(&pattern))
        }
        None => true,
    }
}

pub fn binding_for(vid: u16, pid: u16, hardware_ids: &[String]) -> Option<AdapterBinding> {
    let guard = catalog().read().unwrap();
    let mut candidates: Vec<&AdapterBinding> = guard
        .bindings
        .iter()
        .filter(|b| binding_matches(b, vid, pid, hardware_ids))
        .collect();
    candidates.sort_by(|a, b| {
        b.priority.cmp(&a.priority).then_with(|| {
            a.hardware_id
                .is_some()
                .cmp(&b.hardware_id.is_some())
                .reverse()
        })
    });
    candidates.first().map(|b| (*b).clone())
}

fn entry_matches(entry: &RegistryEntry, vid: u16, pid: u16, hardware_ids: &[String]) -> bool {
    let vid_pid = entry
        .vid_pids
        .iter()
        .filter_map(|text| parse_vid_pid(text))
        .any(|(v, p)| v == vid && p == pid);
    if !vid_pid {
        return false;
    }
    if entry.hardware_ids.is_empty() {
        return true;
    }
    let normalized: Vec<String> = hardware_ids.iter().map(|id| norm(id)).collect();
    entry.hardware_ids.iter().any(|id| {
        let id = norm(id);
        normalized.iter().any(|hw| hw.starts_with(&id))
    })
}

pub fn identify(vid: u16, pid: u16, hardware_ids: &[String]) -> Option<RegistryEntry> {
    let (entries, bindings) = catalog_snapshot();
    if let Some(binding) = bindings
        .iter()
        .filter(|b| binding_matches(b, vid, pid, hardware_ids))
        .max_by_key(|b| b.priority)
    {
        if let Some(registry_id) = &binding.registry_id {
            if let Some(entry) = entries.iter().find(|e| &e.id == registry_id) {
                return Some(entry.clone());
            }
        }
    }
    entries
        .iter()
        .find(|e| entry_matches(e, vid, pid, hardware_ids))
        .cloned()
}

pub fn built_in_bindings() -> Vec<AdapterBinding> {
    let row =
        |id: &str, adapter_id: &str, adapter_label: &str, registry_id: &str, vid: u16, pid: u16| {
            AdapterBinding {
                id: id.into(),
                adapter_id: adapter_id.into(),
                adapter_label: adapter_label.into(),
                vid,
                pid,
                hardware_id: None,
                registry_id: Some(registry_id.into()),
                priority: 10,
                verification: "verified".into(),
                source: "builtin".into(),
            }
        };
    vec![
        row(
            "redsquare-e410",
            "redsquare",
            "Red Square Adapter",
            "redsquare-alumix-tkl",
            0x342D,
            0xE410,
        ),
        row(
            "redsquare-e411",
            "redsquare",
            "Red Square Adapter",
            "redsquare-alumix-tkl",
            0x342D,
            0xE411,
        ),
        row(
            "ajazz-6426",
            "ajazz",
            "AJAZZ Adapter",
            "ajazz-aj159-nl",
            0x7768,
            0x6426,
        ),
        row(
            "ajazz-6d26",
            "ajazz",
            "AJAZZ Adapter",
            "ajazz-aj159-nl",
            0x7768,
            0x6D26,
        ),
        row(
            "ioaudio-4155",
            "ioaudio",
            "IO Audio Adapter",
            "io-graphite-2-pro",
            0x4C4A,
            0x4155,
        ),
    ]
}

fn source(label: &str, url: &str) -> RegistrySource {
    RegistrySource {
        label: label.into(),
        url: url.into(),
    }
}

const USB_IDS: (&str, &str) = (
    "usb.ids — USB ID database",
    "https://www.linux-usb.org/usb-ids.html",
);
const MS_HID: (&str, &str) = (
    "Microsoft — HID report descriptors",
    "https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/hid-report-descriptors",
);
const OPENRGB: (&str, &str) = ("OpenRGB device support", "https://openrgb.org");
const LIBRATBAG: (&str, &str) = (
    "libratbag — gaming mice",
    "https://github.com/libratbag/libratbag",
);
const HEADSETCONTROL: (&str, &str) = ("HeadsetControl", "https://github.com/Sapd/HeadsetControl");
const CORE_AUDIO: (&str, &str) = (
    "Microsoft — Core Audio APIs",
    "https://learn.microsoft.com/en-us/windows/win32/coreaudio/core-audio-interfaces",
);

pub fn built_in_registry() -> Vec<RegistryEntry> {
    vec![
        RegistryEntry {
            id: "redsquare-alumix-tkl".into(),
            vendor: "Red Square".into(),
            model: "Alumix TKL".into(),
            category: "keyboard".into(),
            adapter_id: "redsquare".into(),
            adapter_label: "Red Square Adapter".into(),
            vid_pids: vec!["342D:E410 (USB)".into(), "342D:E411 (2.4G)".into()],
            hardware_ids: vec![
                r"USB\VID_342D&PID_E410".into(),
                r"USB\VID_342D&PID_E411".into(),
            ],
            hardware_revision: None,
            firmware: Vec::new(),
            connections: vec!["USB".into(), "2.4G (RSQ_20055_2.4G)".into()],
            protocol_version: Some("Gsky · feature report 7".into()),
            protocol_ru: "Подсветка: официальный протокол Red Square Gsky. Чтение 0x82, запись 0x02 (feature report 7, кадр 256 байт), подтверждение чтением. Заряд: команда 0x8B по кабелю (проверено 93% на RSQ-20055).".into(),
            protocol_en: "Lighting: official Red Square Gsky protocol. Read 0x82, write 0x02 (feature report 7, 256-byte frame), confirmed by read-back. Battery: command 0x8B over cable (93% verified on RSQ-20055).".into(),
            verification: "verified".into(),
            last_checked: "2026-10-31".into(),
            official_software: Some("https://conf.red-square.org/".into()),
            capabilities: adapters::keyboard_caps(),
            sources: vec![
                source("Red Square — официальный WebHID-конфигуратор", "https://conf.red-square.org/"),
                source(USB_IDS.0, USB_IDS.1),
                source(MS_HID.0, MS_HID.1),
            ],
        },
        RegistryEntry {
            id: "ajazz-aj159-nl".into(),
            vendor: "AJAZZ".into(),
            model: "AJ159 NL".into(),
            category: "mouse".into(),
            adapter_id: "ajazz".into(),
            adapter_label: "AJAZZ Adapter".into(),
            vid_pids: vec!["7768:6426".into(), "7768:6D26".into()],
            hardware_ids: vec![
                r"USB\VID_7768&PID_6426".into(),
                r"USB\VID_7768&PID_6D26".into(),
            ],
            hardware_revision: None,
            firmware: Vec::new(),
            connections: vec!["USB".into(), "NearLink (2.4G)".into()],
            protocol_version: Some("NearLink vendor HID (status frame 06 05)".into()),
            protocol_ru: "Vendor HID NearLink. Чтение статуса 06 05, запись DPI 06 13 05 и частоты 06 13 02 с подтверждением чтением; заряд приходит в кадре статуса. Проверено на 0x7768:0x6426.".into(),
            protocol_en: "NearLink vendor HID. Status read 06 05, DPI write 06 13 05 and polling write 06 13 02 with read-back confirmation; battery arrives in the status frame. Verified on 0x7768:0x6426.".into(),
            verification: "verified".into(),
            last_checked: "2026-10-30".into(),
            official_software: None,
            capabilities: adapters::mouse_caps(),
            sources: vec![
                source(USB_IDS.0, USB_IDS.1),
                source(LIBRATBAG.0, LIBRATBAG.1),
                source(OPENRGB.0, OPENRGB.1),
                source(MS_HID.0, MS_HID.1),
            ],
        },
        RegistryEntry {
            id: "io-graphite-2-pro".into(),
            vendor: "IO".into(),
            model: "Graphite 2 Pro Wireless".into(),
            category: "headset".into(),
            adapter_id: "ioaudio".into(),
            adapter_label: "IO Audio Adapter".into(),
            vid_pids: vec!["4C4A:4155".into()],
            hardware_ids: vec![r"USB\VID_4C4A&PID_4155".into()],
            hardware_revision: None,
            firmware: Vec::new(),
            connections: vec!["USB-ресивер".into()],
            protocol_version: None,
            protocol_ru: "USB Audio + HID. Встроенный DSP не документирован; громкость и баланс идут через Core Audio.".into(),
            protocol_en: "USB Audio + HID. The built-in DSP is undocumented; volume and balance use Core Audio.".into(),
            verification: "experimental".into(),
            last_checked: "2026-10-30".into(),
            official_software: None,
            capabilities: adapters::headset_caps(),
            sources: vec![
                source(USB_IDS.0, USB_IDS.1),
                source(HEADSETCONTROL.0, HEADSETCONTROL.1),
                source(CORE_AUDIO.0, CORE_AUDIO.1),
            ],
        },
    ]
}
