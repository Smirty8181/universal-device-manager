use std::path::PathBuf;

use windows::core::{Interface, GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, IChannelAudioVolume,
    IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    STGM_READ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;
use windows::Win32::UI::Shell::{SHLoadIndirectString, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::model::{AppSession, AudioEndpoint, AudioState};

const PKEY_FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

fn co_init() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
}

fn enumerator() -> Result<IMMDeviceEnumerator, String> {
    co_init();
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.message()) }
}

unsafe fn device_id(device: &IMMDevice) -> Result<String, String> {
    let pw: PWSTR = device.GetId().map_err(|e| e.message())?;
    let value = pw.to_string().unwrap_or_default();
    CoTaskMemFree(Some(pw.0 as *const _));
    Ok(value)
}

unsafe fn endpoint_name(device: &IMMDevice) -> String {
    let Ok(store) = device.OpenPropertyStore(STGM_READ) else {
        return String::new();
    };
    let Ok(value) = store.GetValue(&PKEY_FRIENDLY_NAME) else {
        return String::new();
    };
    value.to_string()
}

fn collect_flow(
    enum_dev: &IMMDeviceEnumerator,
    capture: bool,
) -> Result<Vec<AudioEndpoint>, String> {
    unsafe {
        let flow = if capture { eCapture } else { eRender };
        let collection = enum_dev
            .EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)
            .map_err(|e| e.message())?;
        let default_id = enum_dev
            .GetDefaultAudioEndpoint(flow, eConsole)
            .ok()
            .and_then(|dev| device_id(&dev).ok());
        let count = collection.GetCount().map_err(|e| e.message())?;
        let mut out = Vec::new();
        for index in 0..count {
            let Ok(dev) = collection.Item(index) else {
                continue;
            };
            let Ok(id) = device_id(&dev) else { continue };
            let name = endpoint_name(&dev);
            out.push(AudioEndpoint {
                is_default: default_id.as_deref() == Some(id.as_str()),
                id,
                name,
                direction: if capture { "capture" } else { "render" }.into(),
                state: "Active".into(),
            });
        }
        Ok(out)
    }
}

pub fn list_endpoints() -> Result<Vec<AudioEndpoint>, String> {
    let enum_dev = enumerator()?;
    let mut out = collect_flow(&enum_dev, false)?;
    out.extend(collect_flow(&enum_dev, true)?);
    Ok(out)
}

fn find_device(enum_dev: &IMMDeviceEnumerator, endpoint_id: &str) -> Result<IMMDevice, String> {
    unsafe {
        for flow in [eRender, eCapture] {
            let collection = enum_dev
                .EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)
                .map_err(|e| e.message())?;
            let count = collection.GetCount().map_err(|e| e.message())?;
            for index in 0..count {
                let Ok(dev) = collection.Item(index) else {
                    continue;
                };
                if device_id(&dev).ok().as_deref() == Some(endpoint_id) {
                    return Ok(dev);
                }
            }
        }
    }
    Err("audio endpoint not found".into())
}

fn endpoint_volume(device: &IMMDevice) -> Result<IAudioEndpointVolume, String> {
    unsafe {
        device
            .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
            .map_err(|e| e.message())
    }
}

fn channel_volume(device: &IMMDevice) -> Option<IChannelAudioVolume> {
    unsafe {
        device
            .Activate::<IChannelAudioVolume>(CLSCTX_ALL, None)
            .ok()
    }
}

pub fn endpoint_state(endpoint_id: &str) -> Result<AudioState, String> {
    let enum_dev = enumerator()?;
    let device = find_device(&enum_dev, endpoint_id)?;
    let volume = endpoint_volume(&device)?;
    unsafe {
        let level = volume
            .GetMasterVolumeLevelScalar()
            .map_err(|e| e.message())?;
        let muted = volume.GetMute().map_err(|e| e.message())?.as_bool();
        let count = volume.GetChannelCount().unwrap_or(2).max(1);
        let mut channels = vec![level; count as usize];
        if let Some(v) = channel_volume(&device) {
            if let Ok(cc) = v.GetChannelCount() {
                for i in 0..cc.min(count) {
                    if let Ok(level) = v.GetChannelVolume(i) {
                        channels[i as usize] = level;
                    }
                }
            }
        }
        Ok(AudioState {
            endpoint_id: endpoint_id.to_string(),
            volume: level,
            muted,
            channels,
            channel_count: count,
        })
    }
}

pub fn set_volume(endpoint_id: &str, level: f32) -> Result<(), String> {
    let enum_dev = enumerator()?;
    let device = find_device(&enum_dev, endpoint_id)?;
    let volume = endpoint_volume(&device)?;
    unsafe {
        volume
            .SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null())
            .map_err(|e| e.message())
    }
}

pub fn set_mute(endpoint_id: &str, muted: bool) -> Result<(), String> {
    let enum_dev = enumerator()?;
    let device = find_device(&enum_dev, endpoint_id)?;
    let volume = endpoint_volume(&device)?;
    unsafe {
        volume
            .SetMute(
                windows::Win32::Foundation::BOOL::from(muted),
                std::ptr::null(),
            )
            .map_err(|e| e.message())
    }
}

pub fn set_channel(endpoint_id: &str, index: u32, level: f32) -> Result<(), String> {
    let enum_dev = enumerator()?;
    let device = find_device(&enum_dev, endpoint_id)?;
    let Some(volume) = channel_volume(&device) else {
        return set_volume(endpoint_id, level);
    };
    unsafe {
        volume
            .SetChannelVolume(index, level.clamp(0.0, 1.0), std::ptr::null())
            .map_err(|e| e.message())
    }
}

fn find_by_pattern(
    enum_dev: &IMMDeviceEnumerator,
    capture: bool,
    pattern: Option<&str>,
) -> Option<(String, String)> {
    let list = collect_flow(enum_dev, capture).ok()?;
    if let Some(pattern) = pattern {
        let p = pattern.to_ascii_lowercase();
        if let Some(found) = list
            .iter()
            .find(|e| e.name.to_ascii_lowercase().contains(&p))
        {
            return Some((found.id.clone(), found.name.clone()));
        }
    }
    list.iter()
        .find(|e| e.is_default)
        .or_else(|| list.first())
        .map(|e| (e.id.clone(), e.name.clone()))
}

pub fn apply_profile_audio(
    volume: u8,
    muted: bool,
    channels: Option<&[f32]>,
    render_pattern: Option<&str>,
    capture_pattern: Option<&str>,
    mic_gain: u8,
) -> Result<String, String> {
    let enum_dev = enumerator()?;
    let mut applied = Vec::new();
    if let Some((id, name)) = find_by_pattern(&enum_dev, false, render_pattern) {
        set_volume(&id, volume as f32 / 100.0)?;
        set_mute(&id, muted)?;
        if let Some(channels) = channels {
            for (i, level) in channels.iter().enumerate() {
                let _ = set_channel(&id, i as u32, *level);
            }
        }
        applied.push(name);
    }
    if mic_gain <= 100 {
        if let Some((id, name)) = find_by_pattern(&enum_dev, true, capture_pattern) {
            if set_volume(&id, mic_gain as f32 / 100.0).is_ok() {
                applied.push(name);
            }
        }
    }
    if applied.is_empty() {
        return Err("no audio endpoints available".into());
    }
    Ok(applied.join(" · "))
}

pub fn open_sound_settings() -> Result<(), String> {
    unsafe {
        let op: Vec<u16> = "open\0".encode_utf16().collect();
        let target: Vec<u16> = "ms-settings:sound\0".encode_utf16().collect();
        let result = ShellExecuteW(
            HWND::default(),
            windows::core::PCWSTR(op.as_ptr()),
            windows::core::PCWSTR(target.as_ptr()),
            windows::core::PCWSTR::null(),
            windows::core::PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        if result.0 as isize > 32 {
            Ok(())
        } else {
            Err("failed to open Windows sound settings".into())
        }
    }
}

fn eapo_config_dir() -> Option<PathBuf> {
    let candidates = [
        std::env::var("ProgramFiles")
            .ok()
            .map(|p| PathBuf::from(p).join("EqualizerAPO").join("config")),
        std::env::var("ProgramFiles(x86)")
            .ok()
            .map(|p| PathBuf::from(p).join("EqualizerAPO").join("config")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find(|candidate| candidate.join("config.txt").exists())
}

pub fn eapo_status() -> (bool, String) {
    match eapo_config_dir() {
        Some(dir) => (true, dir.to_string_lossy().into_owned()),
        None => (false, String::new()),
    }
}

fn sanitize_file_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == ' ' {
            out.push(ch);
        }
    }
    let trimmed = out.trim();
    if trimmed.is_empty() {
        "Preset".into()
    } else {
        trimmed.chars().take(60).collect()
    }
}

pub fn eapo_save_preset(name: &str, lines: &[String]) -> Result<String, String> {
    let Some(dir) = eapo_config_dir() else {
        return Err("Equalizer APO is not installed".into());
    };
    let file_name = format!("UDM {}.txt", sanitize_file_name(name));
    let path = dir.join(&file_name);
    let body = format!(
        "# Equalizer APO preset written by Universal Device Manager\r\n# Apply it manually via Include in config.txt\r\n{}\r\n",
        lines.join("\r\n")
    );
    std::fs::write(&path, body).map_err(|e| e.to_string())?;
    crate::store::log_line(&format!("EAPO preset saved: {}", path.display()));
    Ok(path.to_string_lossy().into_owned())
}

fn process_name(pid: u32) -> String {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buffer = [0u16; 512];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        if result.is_err() {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        path.rsplit('\\').next().unwrap_or_default().to_string()
    }
}

fn session_manager(device: &IMMDevice) -> Result<IAudioSessionManager2, String> {
    unsafe {
        device
            .Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
            .map_err(|e| e.message())
    }
}

fn read_string(getter: impl FnOnce() -> windows::core::Result<PWSTR>) -> String {
    getter()
        .map(|pw| unsafe {
            let value = pw.to_string().unwrap_or_default();
            CoTaskMemFree(Some(pw.0 as *const _));
            value
        })
        .unwrap_or_default()
}

/// Session display names arrive as shell indirection references like "@%SystemRoot%\\..."
fn indirect_text(source: &str) -> String {
    let wide: Vec<u16> = source.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buffer = [0u16; 1024];
    unsafe {
        if SHLoadIndirectString(PCWSTR(wide.as_ptr()), &mut buffer, None).is_ok() {
            let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
            String::from_utf16_lossy(&buffer[..len])
        } else {
            String::new()
        }
    }
}

fn collect_sessions(endpoint_id: &str) -> Result<Vec<(String, IAudioSessionControl2)>, String> {
    let enum_dev = enumerator()?;
    let device = find_device(&enum_dev, endpoint_id)?;
    let manager = session_manager(&device)?;
    unsafe {
        let enumerator = manager.GetSessionEnumerator().map_err(|e| e.message())?;
        let count = enumerator.GetCount().map_err(|e| e.message())?;
        let mut out = Vec::new();
        for index in 0..count {
            let Ok(control) = enumerator.GetSession(index) else {
                continue;
            };
            let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
                continue;
            };
            let id = read_string(|| control2.GetSessionInstanceIdentifier());
            if !id.is_empty() {
                out.push((id, control2));
            }
        }
        Ok(out)
    }
}

pub fn sessions(endpoint_id: &str) -> Result<Vec<AppSession>, String> {
    let list = collect_sessions(endpoint_id)?;
    let mut out = Vec::with_capacity(list.len());
    for (id, control2) in &list {
        let pid = unsafe { control2.GetProcessId() }.unwrap_or(0);
        let display = read_string(|| unsafe { control2.GetDisplayName() });
        let simple: windows::core::Result<windows::Win32::Media::Audio::ISimpleAudioVolume> =
            control2.cast();
        let Ok(simple) = simple else { continue };
        let volume = unsafe { simple.GetMasterVolume() }.unwrap_or(0.0);
        let muted = unsafe { simple.GetMute() }
            .map(|b| b.as_bool())
            .unwrap_or(false);
        let system_sounds = unsafe { control2.IsSystemSoundsSession() }.0 == 0;
        let mut name = if display.starts_with('@') {
            indirect_text(&display)
        } else {
            display.clone()
        };
        if name.is_empty() {
            name = if pid > 0 {
                process_name(pid)
            } else if system_sounds {
                "System sounds".into()
            } else {
                "unknown".into()
            };
        }
        out.push(AppSession {
            instance_id: id.clone(),
            name,
            process_id: pid,
            volume,
            muted,
            system_sounds,
        });
    }
    out.sort_by(|a, b| {
        b.volume
            .partial_cmp(&a.volume)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}

fn session_volume(
    endpoint_id: &str,
    instance_id: &str,
) -> Result<windows::Win32::Media::Audio::ISimpleAudioVolume, String> {
    let list = collect_sessions(endpoint_id)?;
    let (_, control2) = list
        .into_iter()
        .find(|(id, _)| id == instance_id)
        .ok_or("audio session not found")?;
    control2.cast().map_err(|e| e.message())
}

pub fn set_session_volume(endpoint_id: &str, instance_id: &str, level: f32) -> Result<(), String> {
    let volume = session_volume(endpoint_id, instance_id)?;
    unsafe {
        volume
            .SetMasterVolume(level.clamp(0.0, 1.0), std::ptr::null())
            .map_err(|e| e.message())
    }
}

pub fn set_session_mute(endpoint_id: &str, instance_id: &str, muted: bool) -> Result<(), String> {
    let volume = session_volume(endpoint_id, instance_id)?;
    unsafe {
        volume
            .SetMute(
                windows::Win32::Foundation::BOOL::from(muted),
                std::ptr::null(),
            )
            .map_err(|e| e.message())
    }
}
