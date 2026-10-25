use std::ffi::c_void;
use std::io::{Read, Seek, SeekFrom};

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::model::SystemInfo;
use crate::store::AppState;

#[repr(C)]
struct OsVersionInfo {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    csd: [u16; 128],
}

#[link(name = "ntdll")]
extern "system" {
    fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
}

pub fn os_version() -> String {
    let mut info = OsVersionInfo {
        size: std::mem::size_of::<OsVersionInfo>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        csd: [0; 128],
    };
    unsafe {
        let _ = RtlGetVersion(&mut info);
    }
    if info.major == 0 {
        return "Windows".into();
    }
    let name = if info.major == 10 && info.build >= 22000 {
        "Windows 11"
    } else if info.major == 10 {
        "Windows 10"
    } else {
        "Windows"
    };
    format!(
        "{} · {}.{} build {}",
        name, info.major, info.minor, info.build
    )
}

pub fn is_admin() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut c_void),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

pub fn info(state: &AppState, app_version: &str) -> SystemInfo {
    let devices = state.devices.lock().unwrap().clone();
    let adapter_count = devices.iter().filter(|d| d.adapter_id != "generic").count();
    SystemInfo {
        app_version: app_version.into(),
        os_version: os_version(),
        admin: is_admin(),
        data_dir: state.data_dir.to_string_lossy().into_owned(),
        log_file: if state.log_path.exists() {
            Some(state.log_path.to_string_lossy().into_owned())
        } else {
            None
        },
        device_count: devices.len(),
        adapter_count,
        started_at: state.started_at,
        tauri: true,
    }
}

pub fn log_tail(lines: usize) -> String {
    let path = crate::store::log_path();
    let Ok(mut file) = std::fs::File::open(path) else {
        return String::new();
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(256 * 1024);
    let _ = file.seek(SeekFrom::Start(start));
    let mut buf = String::new();
    let _ = file.read_to_string(&mut buf);
    let all: Vec<&str> = buf.lines().collect();
    let take = lines.clamp(1, 2000).min(all.len());
    all[all.len() - take..].join("\n")
}

pub fn copy_text(text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_UNICODETEXT;
    unsafe {
        OpenClipboard(None).map_err(|e| e.message())?;
        let result = (|| -> Result<(), String> {
            EmptyClipboard().map_err(|e| e.message())?;
            let mut units: Vec<u16> = text.encode_utf16().collect();
            units.push(0);
            let hmem = GlobalAlloc(GMEM_MOVEABLE, units.len() * 2).map_err(|e| e.message())?;
            let ptr = GlobalLock(hmem);
            if ptr.is_null() {
                return Err("GlobalLock failed".into());
            }
            std::ptr::copy_nonoverlapping(units.as_ptr(), ptr as *mut u16, units.len());
            let _ = GlobalUnlock(hmem);
            SetClipboardData(CF_UNICODETEXT.0 as u32, HANDLE(hmem.0)).map_err(|e| e.message())?;
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}
