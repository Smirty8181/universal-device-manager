use std::os::windows::process::CommandExt;
use std::process::Command;

use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::Foundation::{FILETIME, HWND};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

use crate::drivers::{scan_internal, signer_of_path, wide_to_string};
use crate::model::{now_ms, DriverCandidate, DriverResolution, FirmwareInfo, RegistryLink};
use crate::store::AppState;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn packed_version(raw: u64) -> String {
    [
        (raw >> 48) & 0xffff,
        (raw >> 32) & 0xffff,
        (raw >> 16) & 0xffff,
        raw & 0xffff,
    ]
    .iter()
    .map(|part| part.to_string())
    .collect::<Vec<_>>()
    .join(".")
}

fn version_parts(text: &str) -> Vec<u64> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn version_newer(candidate: &str, installed: &str) -> bool {
    let left = version_parts(candidate);
    let right = version_parts(installed);
    if left.is_empty() || right.is_empty() {
        return false;
    }
    let depth = left.len().max(right.len());
    for i in 0..depth {
        let a = *left.get(i).unwrap_or(&0);
        let b = *right.get(i).unwrap_or(&0);
        if a != b {
            return a > b;
        }
    }
    false
}

fn filetime_date(value: FILETIME) -> Option<String> {
    let ticks = ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64;
    let epoch_ticks = 116_444_736_000_000_000; // 1 Jan 1601 in 100ns units
    if ticks < epoch_ticks {
        return None;
    }
    let ms = (ticks - epoch_ticks) / 10_000;
    chrono::DateTime::from_timestamp_millis(ms as i64).map(|dt| dt.format("%d.%m.%Y").to_string())
}

fn inf_text(path: &str) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 4_000_000 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let words: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        Some(String::from_utf16_lossy(&words).to_ascii_uppercase())
    } else {
        Some(String::from_utf8_lossy(&bytes).to_ascii_uppercase())
    }
}

fn claims_hwid(path: &str, wanted: &[String]) -> bool {
    let Some(text) = inf_text(path) else {
        return false;
    };
    wanted.iter().any(|id| {
        let upper = id.to_ascii_uppercase();
        upper.len() > 8 && text.contains(&upper)
    })
}

unsafe fn store_candidates(instance: &str, hardware_ids: &[String]) -> Vec<DriverCandidate> {
    let mut out = Vec::new();
    let Ok(set) = SetupDiGetClassDevsW(
        None,
        PCWSTR::null(),
        HWND::default(),
        DIGCF_PRESENT | DIGCF_ALLCLASSES,
    ) else {
        crate::store::log_line("resolve step class-devs failed");
        return out;
    };
    let mut data = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };
    let mut found = false;
    let mut scan = 0u32;
    while !found {
        let mut device = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        if SetupDiEnumDeviceInfo(set, scan, &mut device).is_err() {
            break;
        }
        scan += 1;
        let mut buf = [0u16; 512];
        if SetupDiGetDeviceInstanceIdW(set, &device, Some(&mut buf), None).is_err() {
            continue;
        }
        if wide_to_string(&buf).eq_ignore_ascii_case(instance) {
            data = device;
            found = true;
        }
    }
    if !found {
        crate::store::log_line(&format!("resolve step device not present: {instance}"));
        let _ = SetupDiDestroyDeviceInfoList(set);
        return out;
    }
    if let Err(error) = SetupDiBuildDriverInfoList(set, Some(&mut data), SPDIT_COMPATDRIVER) {
        crate::store::log_line(&format!("resolve step build-driver-list: {error}"));
        let _ = SetupDiDestroyDeviceInfoList(set);
        return out;
    }
    let mut index = 0u32;
    while index < 256 {
        let mut info = SP_DRVINFO_DATA_V2_W {
            cbSize: std::mem::size_of::<SP_DRVINFO_DATA_V2_W>() as u32,
            ..Default::default()
        };
        if SetupDiEnumDriverInfoW(set, Some(&data), SPDIT_COMPATDRIVER, index, &mut info).is_err() {
            break;
        }
        index += 1;
        // the detail struct is followed by a variable-length ID list, so the buffer must be over-allocated
        let words = std::mem::size_of::<SP_DRVINFO_DETAIL_DATA_W>() / 8 + 2048;
        let mut storage = vec![0u64; words];
        let detail_ptr = storage.as_mut_ptr() as *mut SP_DRVINFO_DETAIL_DATA_W;
        (*detail_ptr).cbSize = std::mem::size_of::<SP_DRVINFO_DETAIL_DATA_W>() as u32;
        if SetupDiGetDriverInfoDetailW(
            set,
            Some(&data),
            &info,
            Some(detail_ptr),
            (storage.len() * 8) as u32,
            None,
        )
        .is_err()
        {
            continue;
        }
        let detail = &*detail_ptr;
        let path = wide_to_string(&detail.InfFileName);
        if path.is_empty() {
            continue;
        }
        let file_name = std::path::Path::new(&path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();
        let published = if file_name.to_ascii_lowercase().starts_with("oem") {
            Some(file_name)
        } else {
            None
        };
        let signature = signer_of_path(&path);
        let exact = claims_hwid(&path, hardware_ids);
        out.push(DriverCandidate {
            inf_path: path,
            published_name: published,
            description: wide_to_string(&info.Description),
            provider: wide_to_string(&info.ProviderName),
            version: packed_version(info.DriverVersion),
            date: filetime_date(info.DriverDate),
            class_name: String::new(),
            signature,
            exact_hwid_match: exact,
            newer_than_installed: false,
            is_installed: false,
        });
    }
    let _ = SetupDiDestroyDeviceInfoList(set);
    out
}

fn url_encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn registry_links(state: &AppState, hardware_ids: &[String]) -> Vec<RegistryLink> {
    let mut links = Vec::new();
    let record = state
        .db
        .as_ref()
        .and_then(|db| db.lock().ok())
        .and_then(|conn| crate::db::match_registry(&conn, hardware_ids));
    if let Some(record) = record {
        for source in record.sources {
            links.push(RegistryLink {
                label: source.label,
                url: source.url,
            });
        }
        if let Some(software) = record.official_software {
            if software.starts_with("http") {
                links.push(RegistryLink {
                    label: format!("{} {}", record.vendor, record.model),
                    url: software,
                });
            }
        }
    }
    let first = hardware_ids.first().cloned().unwrap_or_default();
    if !first.is_empty() {
        links.push(RegistryLink {
            label: "Microsoft Update Catalog".into(),
            url: format!(
                "https://www.catalog.update.microsoft.com/Search.aspx?q={}",
                url_encode(&first)
            ),
        });
    }
    let mut unique: Vec<String> = Vec::new();
    links.retain(|link| {
        if unique.contains(&link.url) {
            false
        } else {
            unique.push(link.url.clone());
            true
        }
    });
    links
}

pub fn resolve(state: &AppState, instance: &str) -> Result<DriverResolution, String> {
    crate::store::log_line(&format!("driver resolve started: {instance}"));
    let mut found = scan_internal(Some(instance));
    if found.is_empty() {
        return Err(format!("device not found: {instance}"));
    }
    let entry = found.remove(0);
    let candidates = unsafe { store_candidates(instance, &entry.hardware_ids) };

    let mut seen: Vec<String> = Vec::new();
    let candidates: Vec<DriverCandidate> = candidates
        .into_iter()
        .filter(|c| {
            let key = c.inf_path.to_ascii_lowercase();
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        })
        .map(|mut c| {
            c.class_name = entry.class_name.clone();
            c.newer_than_installed = match entry.driver_version.as_deref() {
                Some(installed) => version_newer(&c.version, installed),
                None => false,
            };
            c.is_installed = entry.driver_inf.as_deref() == c.published_name.as_deref()
                || entry.driver_version.as_deref() == Some(c.version.as_str());
            c
        })
        .collect();

    let mut candidates = candidates;
    candidates.sort_by(|a, b| {
        b.exact_hwid_match
            .cmp(&a.exact_hwid_match)
            .then_with(|| b.newer_than_installed.cmp(&a.newer_than_installed))
            .then_with(|| version_parts(&b.version).cmp(&version_parts(&a.version)))
    });

    crate::store::log_line(&format!(
        "driver resolve finished: {} candidates",
        candidates.len()
    ));
    let sources = registry_links(state, &entry.hardware_ids);
    Ok(DriverResolution {
        instance_id: entry.instance_id.clone(),
        device_name: entry.name,
        hardware_ids: entry.hardware_ids,
        installed_version: entry.driver_version,
        installed_inf: entry.driver_inf,
        candidates,
        sources,
    })
}

fn run_pnputil(args: &[&str]) -> Result<String, String> {
    let output = Command::new("pnputil.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let text = text.trim().to_string();
    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        crate::store::log_line(&format!("pnputil failed with code {code}: {text}"));
        return Err(format!("pnputil exited with code {code}"));
    }
    Ok(text)
}

fn require_signed(inf_path: &str) -> Result<String, String> {
    let path = std::path::Path::new(inf_path);
    if !path.is_file() {
        return Err("driver package is missing".into());
    }
    let is_inf = path
        .extension()
        .map(|e| e.to_string_lossy().eq_ignore_ascii_case("inf"))
        .unwrap_or(false);
    if !is_inf {
        return Err("only .inf driver packages can be installed".into());
    }
    let signer = signer_of_path(inf_path);
    if signer != "windows" && signer != "publisher" {
        crate::store::log_line(&format!("driver install refused ({signer}): {inf_path}"));
        return Err(format!("signature check failed: {signer}"));
    }
    Ok(signer)
}

pub fn install(inf_path: &str, confirm: bool) -> Result<String, String> {
    if !confirm {
        return Err("installation requires an explicit confirmation".into());
    }
    let signer = require_signed(inf_path)?;
    crate::store::log_line(&format!("driver install started ({signer}): {inf_path}"));
    let text = run_pnputil(&["/add-driver", inf_path, "/install"])?;
    crate::store::log_line(&format!("driver install finished: {inf_path}"));
    Ok(text)
}

pub fn install_elevated(inf_path: &str, confirm: bool) -> Result<String, String> {
    if !confirm {
        return Err("installation requires an explicit confirmation".into());
    }
    require_signed(inf_path)?;
    let params = format!("/add-driver \"{inf_path}\" /install");
    let params_wide = wide(&params);
    let result = unsafe {
        ShellExecuteW(
            HWND::default(),
            PCWSTR(wide("runas").as_ptr()),
            PCWSTR(wide("pnputil.exe").as_ptr()),
            PCWSTR(params_wide.as_ptr()),
            PCWSTR::null(),
            SW_SHOW,
        )
    };
    if (result.0 as isize) <= 32 {
        return Err(format!(
            "pnputil could not be started (code {})",
            result.0 as isize
        ));
    }
    crate::store::log_line(&format!("driver install elevation requested: {inf_path}"));
    Ok("elevation requested — confirm the Windows prompt".into())
}

pub fn record_install(
    state: &AppState,
    instance_id: &str,
    name: &str,
    result: &str,
) -> Result<(), String> {
    {
        let mut history = state.driver_history.lock().unwrap();
        history.insert(
            0,
            crate::model::DriverHistoryEntry {
                at: now_ms(),
                instance_id: instance_id.to_string(),
                name: name.to_string(),
                result: result.to_string(),
            },
        );
        history.truncate(100);
    }
    state.save_driver_history()
}

pub fn device_name(instance: &str) -> String {
    scan_internal(Some(instance))
        .into_iter()
        .next()
        .map(|entry| entry.name)
        .unwrap_or_else(|| instance.to_string())
}

fn date_token(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .find(|token| {
            token.chars().filter(|c| c.is_ascii_digit()).count() == 8
                && token.chars().filter(|c| ".-/".contains(*c)).count() == 2
        })
        .map(|token| token.to_string())
}

fn version_token(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .find(|token| {
            token.contains('.')
                && token.chars().all(|c| c.is_ascii_digit() || c == '.')
                && token.chars().filter(|c| *c == '.').count() >= 2
        })
        .map(|token| token.to_string())
}

fn is_published_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.ends_with(".inf") && lower.starts_with("oem")
}

fn parse_store(text: &str) -> Vec<crate::model::StoreDriver> {
    let mut out: Vec<crate::model::StoreDriver> = Vec::new();
    let mut record: Option<crate::model::StoreDriver> = None;
    for line in text.lines() {
        let Some((_, value)) = line.trim().split_once(':') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if is_published_name(value) {
            if let Some(done) = record.take() {
                out.push(done);
            }
            record = Some(crate::model::StoreDriver {
                published_name: value.to_string(),
                original_name: String::new(),
                provider: String::new(),
                class_name: String::new(),
                version: String::new(),
                date: None,
            });
            continue;
        }
        let Some(current) = record.as_mut() else {
            continue;
        };
        // pnputil localises the field names, so only the shape of the value says what it is.
        let date = date_token(value);
        let version = version_token(value);
        if date.is_some() || version.is_some() {
            if current.date.is_none() {
                current.date = date;
            }
            if current.version.is_empty() {
                current.version = version.unwrap_or_default();
            }
            continue;
        }
        if value.ends_with(".inf") {
            if current.original_name.is_empty() {
                current.original_name = value.to_string();
            }
            continue;
        }
        if value.starts_with('{') {
            continue;
        }
        if current.provider.is_empty() {
            current.provider = value.to_string();
        } else if current.class_name.is_empty() {
            current.class_name = value.to_string();
        }
    }
    if let Some(done) = record {
        out.push(done);
    }
    out
}

pub fn store_list() -> Result<Vec<crate::model::StoreDriver>, String> {
    let text = run_pnputil(&["/enum-drivers"])?;
    let drivers = parse_store(&text);
    crate::store::log_line(&format!(
        "driver store enumerated: {} packages",
        drivers.len()
    ));
    Ok(drivers)
}

fn revision_of(text: &str) -> Option<String> {
    let upper = text.to_ascii_uppercase();
    let pos = upper.find("REV_")?;
    let tail = upper.get(pos..)?;
    let value: String = tail.chars().take(8).collect();
    if value.len() == 8 {
        Some(value.to_ascii_lowercase())
    } else {
        None
    }
}

pub fn firmware(state: &AppState, instance: &str) -> Result<FirmwareInfo, String> {
    let mut found = scan_internal(Some(instance));
    if found.is_empty() {
        return Err(format!("device not found: {instance}"));
    }
    let entry = found.remove(0);
    let revision = revision_of(&entry.instance_id)
        .or_else(|| entry.hardware_ids.iter().find_map(|id| revision_of(id)));
    let record = state
        .db
        .as_ref()
        .and_then(|db| db.lock().ok())
        .and_then(|conn| crate::db::match_registry(&conn, &entry.hardware_ids));
    let known_revisions = record
        .as_ref()
        .map(|r| r.firmware.clone())
        .unwrap_or_default();
    let protocol_version = record.as_ref().and_then(|r| {
        r.protocol_version
            .clone()
            .or_else(|| r.hardware_revision.clone())
    });
    let sources = registry_links(state, &entry.hardware_ids);
    Ok(FirmwareInfo {
        instance_id: entry.instance_id.clone(),
        device_name: entry.name,
        hardware_ids: entry.hardware_ids,
        revision,
        protocol_version,
        driver_version: entry.driver_version,
        driver_date: entry.driver_date,
        signature: entry.signature,
        known_revisions,
        sources,
        // flashing is off limits until a vendor documents a recovery path
        flashable: false,
        recovery_documented: false,
    })
}
