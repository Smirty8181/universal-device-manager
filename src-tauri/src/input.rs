use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyNameTextW, MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD,
    INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
    MAPVK_VK_TO_VSC, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
    MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_XDOWN,
    MOUSEEVENTF_XUP, MOUSEINPUT, MOUSE_EVENT_FLAGS, VIRTUAL_KEY, VK_RCONTROL, VK_RSHIFT,
};
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, RegisterRawInputDevices, HRAWINPUT, RAWINPUT,
    RAWINPUTDEVICE, RAWINPUTHEADER, RIDEV_INPUTSINK, RIDI_DEVICENAME, RID_INPUT, RIM_TYPEHID,
    RIM_TYPEKEYBOARD, RIM_TYPEMOUSE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetSystemMetrics,
    PeekMessageW, RegisterClassExW, SendMessageTimeoutW, SetWindowsHookExW, SystemParametersInfoW,
    TranslateMessage, HHOOK, HWND_BROADCAST, HWND_MESSAGE, KBDLLHOOKSTRUCT, MSG, MSLLHOOKSTRUCT,
    PM_REMOVE, SMTO_ABORTIFHUNG, SM_SWAPBUTTON, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_GETMOUSE,
    SPI_GETMOUSESPEED, SPI_GETWHEELSCROLLLINES, SPI_SETMOUSE, SPI_SETMOUSEBUTTONSWAP,
    SPI_SETMOUSESPEED, SPI_SETWHEELSCROLLLINES, SYSTEM_PARAMETERS_INFO_ACTION,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WH_KEYBOARD_LL, WH_MOUSE_LL, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_INPUT, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN,
    WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETTINGCHANGE, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSEXW,
};

use crate::model::{InputMap, InterceptEvent, MouseSystem, PollingStat};

const KEY_MARK: usize = 0x5544_4D01;
const MOUSE_MARK: usize = 0x5544_4D02;
pub const MACRO_MARK: usize = 0x5544_4D03;
const WINDOW_CLASS: &str = "UDMRawInputWindow";
const PANIC_HOLD: Duration = Duration::from_secs(3);

static APP: OnceLock<AppHandle> = OnceLock::new();
static KB_HOOK: AtomicIsize = AtomicIsize::new(0);
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
static ENGINE_ACTIVE: AtomicBool = AtomicBool::new(false);
static REMAP_ON: AtomicBool = AtomicBool::new(false);
static GAME_MODE: AtomicBool = AtomicBool::new(false);

static KEYS: OnceLock<Mutex<HashMap<u32, Option<u32>>>> = OnceLock::new();
static BUTTONS: OnceLock<Mutex<HashMap<u32, (String, u32)>>> = OnceLock::new();
type ReportMap = HashMap<String, (String, VecDeque<u64>)>;

static REPORTS: OnceLock<Mutex<ReportMap>> = OnceLock::new();
static NAMES: OnceLock<Mutex<HashMap<usize, String>>> = OnceLock::new();
static PANIC_SINCE: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
static CLOCK: OnceLock<Instant> = OnceLock::new();
static RAW_BUF: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();
static SEEN_KEYS: AtomicIsize = AtomicIsize::new(0);
static SWALLOWED_KEYS: AtomicIsize = AtomicIsize::new(0);
static SEEN_BUTTONS: AtomicIsize = AtomicIsize::new(0);
static SWALLOWED_BUTTONS: AtomicIsize = AtomicIsize::new(0);
static RECENT: OnceLock<Mutex<VecDeque<InterceptEvent>>> = OnceLock::new();

static LIVE_ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE_TOTAL: AtomicU64 = AtomicU64::new(0);
static LIVE_LAST: OnceLock<Mutex<Option<u64>>> = OnceLock::new();
type LiveCountMap = HashMap<String, (u64, u64)>;
type LiveLabelMap = HashMap<String, (String, Option<String>)>;

static LIVE_COUNTS: OnceLock<Mutex<LiveCountMap>> = OnceLock::new();
static LIVE_PRESSED: OnceLock<Mutex<HashSet<(String, u32)>>> = OnceLock::new();
static LIVE_LABELS: OnceLock<Mutex<LiveLabelMap>> = OnceLock::new();

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LiveKeyEvent {
    pub at: u64,
    pub vk: u32,
    pub scan: u32,
    pub extended: bool,
    pub down: bool,
    pub repeat: bool,
    pub path: String,
    pub label: String,
    pub device_id: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveSource {
    pub path: String,
    pub label: String,
    pub device_id: Option<String>,
    pub events: u64,
    pub last_at: Option<u64>,
    pub pressed: Vec<u32>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveStatus {
    pub active: bool,
    pub total_events: u64,
    pub last_event_at: Option<u64>,
    pub sources: Vec<LiveSource>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardSourceView {
    pub device_id: Option<String>,
    pub name: String,
    pub connection_label: String,
    pub online: bool,
    pub managed: bool,
    pub receiver_shared: bool,
    pub interfaces: usize,
    pub paths: Vec<String>,
}

fn live_counts() -> &'static Mutex<HashMap<String, (u64, u64)>> {
    LIVE_COUNTS.get_or_init(Default::default)
}

fn live_pressed() -> &'static Mutex<HashSet<(String, u32)>> {
    LIVE_PRESSED.get_or_init(Default::default)
}

fn live_labels() -> &'static Mutex<HashMap<String, (String, Option<String>)>> {
    LIVE_LABELS.get_or_init(Default::default)
}

fn live_last() -> &'static Mutex<Option<u64>> {
    LIVE_LAST.get_or_init(Default::default)
}

pub fn path_key(path: &str) -> String {
    let lower = path.trim_start_matches("\\\\?\\").to_ascii_lowercase();
    let base = lower.split("#{").next().unwrap_or(&lower).to_string();
    let mut parts: Vec<String> = base.split('#').map(|part| part.to_string()).collect();
    if parts.len() >= 2 {
        if let Some(pos) = parts[1].find("&col") {
            parts[1].truncate(pos);
        }
    }
    parts.join("#")
}

fn normalize_path(path: &str) -> String {
    path_key(path)
}

fn fallback_label(path: &str) -> String {
    let (vid, pid) = parse_vid_pid(path);
    match (vid, pid) {
        (Some(vid), Some(pid)) => format!("HID keyboard {:04X}:{:04X}", vid, pid),
        _ => "HID keyboard".into(),
    }
}

fn source_meta(path: &str) -> (String, Option<String>) {
    if let Some((label, id)) = live_labels().lock().unwrap().get(&normalize_path(path)) {
        return (label.clone(), id.clone());
    }
    (fallback_label(path), None)
}

pub fn set_source_map(entries: Vec<(String, String, Option<String>)>) {
    let mut map = live_labels().lock().unwrap();
    map.clear();
    for (path, label, device_id) in entries {
        if path.trim().is_empty() {
            continue;
        }
        map.insert(normalize_path(&path), (label, device_id));
    }
}

fn publish_live(path: &str, vk: u32, scan: u32, extended: bool, down: bool) {
    if !LIVE_ACTIVE.load(Ordering::SeqCst) {
        return;
    }
    let key = (normalize_path(path), vk);
    let repeat = {
        let mut pressed = live_pressed().lock().unwrap();
        if down {
            !pressed.insert(key)
        } else {
            pressed.remove(&key);
            false
        }
    };
    let at = crate::model::now_ms();
    if down {
        let mut counts = live_counts().lock().unwrap();
        let entry = counts.entry(normalize_path(path)).or_insert((0, at));
        entry.0 = entry.0.saturating_add(1);
        entry.1 = at;
    }
    LIVE_TOTAL.fetch_add(1, Ordering::Relaxed);
    *live_last().lock().unwrap() = Some(at);
    if let Some(app) = APP.get() {
        let (label, device_id) = source_meta(path);
        let _ = app.emit(
            "keyboard:event",
            LiveKeyEvent {
                at,
                vk,
                scan,
                extended,
                down,
                repeat,
                path: path.to_string(),
                label,
                device_id,
            },
        );
    }
}

pub fn live_status() -> LiveStatus {
    let active = LIVE_ACTIVE.load(Ordering::SeqCst);
    let total_events = LIVE_TOTAL.load(Ordering::Relaxed);
    let last_event_at = *live_last().lock().unwrap();
    let counts = live_counts().lock().unwrap().clone();
    let pressed = live_pressed().lock().unwrap().clone();
    let mut sources: Vec<LiveSource> = counts
        .into_iter()
        .map(|(path, (events, last_at))| {
            let (label, device_id) = source_meta(&path);
            let mut keys: Vec<u32> = pressed
                .iter()
                .filter(|(source, _)| *source == path)
                .map(|(_, vk)| *vk)
                .collect();
            keys.sort_unstable();
            LiveSource {
                path,
                label,
                device_id,
                events,
                last_at: Some(last_at),
                pressed: keys,
            }
        })
        .collect();
    sources.sort_by_key(|source| std::cmp::Reverse(source.last_at));
    LiveStatus {
        active,
        total_events,
        last_event_at,
        sources,
    }
}

pub fn set_live(app: &AppHandle, enabled: bool) -> LiveStatus {
    LIVE_ACTIVE.store(enabled, Ordering::SeqCst);
    live_counts().lock().unwrap().clear();
    live_pressed().lock().unwrap().clear();
    LIVE_TOTAL.store(0, Ordering::Relaxed);
    *live_last().lock().unwrap() = None;
    crate::store::log_line(&format!(
        "keyboard visualizer capture: {}",
        if enabled { "started" } else { "stopped" }
    ));
    let status = live_status();
    let _ = app.emit("keyboard:live", &status);
    status
}

fn recent() -> &'static Mutex<VecDeque<InterceptEvent>> {
    RECENT.get_or_init(Default::default)
}

fn record(kind: &str, from: u32, to: Option<u32>, action: &str) {
    let mut list = recent().lock().unwrap();
    list.push_back(InterceptEvent {
        at: crate::model::now_ms(),
        kind: kind.to_string(),
        from,
        to,
        action: action.to_string(),
    });
    while list.len() > 24 {
        list.pop_front();
    }
}

pub fn history() -> Vec<InterceptEvent> {
    recent().lock().unwrap().iter().rev().cloned().collect()
}

fn raw_buf() -> &'static Mutex<Vec<u8>> {
    RAW_BUF.get_or_init(Default::default)
}

fn names() -> &'static Mutex<HashMap<usize, String>> {
    NAMES.get_or_init(Default::default)
}

fn panic_since() -> &'static Mutex<Option<Instant>> {
    PANIC_SINCE.get_or_init(Default::default)
}

fn is_own_injection(extra: usize) -> bool {
    matches!(extra, KEY_MARK | MOUSE_MARK | MACRO_MARK)
}

fn stamp() -> u64 {
    CLOCK.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn keys() -> &'static Mutex<HashMap<u32, Option<u32>>> {
    KEYS.get_or_init(Default::default)
}

fn buttons() -> &'static Mutex<HashMap<u32, (String, u32)>> {
    BUTTONS.get_or_init(Default::default)
}

fn reports() -> &'static Mutex<HashMap<String, (String, VecDeque<u64>)>> {
    REPORTS.get_or_init(Default::default)
}

pub fn active() -> bool {
    ENGINE_ACTIVE.load(Ordering::SeqCst)
}

pub fn remapping() -> bool {
    REMAP_ON.load(Ordering::SeqCst)
}

pub fn set_game_mode(enabled: bool) {
    GAME_MODE.store(enabled, Ordering::SeqCst);
    crate::store::log_line(&format!(
        "game mode: {}",
        if enabled {
            "windows key blocked"
        } else {
            "windows key released"
        }
    ));
}

pub fn apply(map: &InputMap) {
    let mut table = keys().lock().unwrap();
    table.clear();
    for entry in &map.keys {
        table.insert(entry.vk, entry.target);
    }
    drop(table);
    let mut mouse = buttons().lock().unwrap();
    mouse.clear();
    for entry in &map.buttons {
        mouse.insert(entry.button, (entry.action.clone(), entry.target));
    }
    drop(mouse);
    let armed = map.enabled && !(map.keys.is_empty() && map.buttons.is_empty());
    REMAP_ON.store(armed, Ordering::SeqCst);
}

fn is_extended(vk: u32) -> bool {
    matches!(
        vk,
        0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x5D | 0x1C | 0x6F | 0x90 | 0x2A
    )
}

unsafe fn inject_key(vk: u32, down: bool) {
    let mut flags = if down {
        KEYBD_EVENT_FLAGS(0)
    } else {
        KEYEVENTF_KEYUP
    };
    if is_extended(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk as u16),
                wScan: MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) as u16,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: KEY_MARK,
            },
        },
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}

fn mouse_parts(button: u32, down: bool) -> Option<(MOUSE_EVENT_FLAGS, u32)> {
    match (button, down) {
        (0, true) => Some((MOUSEEVENTF_LEFTDOWN, 0)),
        (0, false) => Some((MOUSEEVENTF_LEFTUP, 0)),
        (1, true) => Some((MOUSEEVENTF_RIGHTDOWN, 0)),
        (1, false) => Some((MOUSEEVENTF_RIGHTUP, 0)),
        (2, true) => Some((MOUSEEVENTF_MIDDLEDOWN, 0)),
        (2, false) => Some((MOUSEEVENTF_MIDDLEUP, 0)),
        (3, true) => Some((MOUSEEVENTF_XDOWN, 1)),
        (3, false) => Some((MOUSEEVENTF_XUP, 1)),
        (4, true) => Some((MOUSEEVENTF_XDOWN, 2)),
        (4, false) => Some((MOUSEEVENTF_XUP, 2)),
        _ => None,
    }
}

unsafe fn inject_button(button: u32, down: bool) {
    let Some((flags, data)) = mouse_parts(button, down) else {
        return;
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: MOUSE_MARK,
            },
        },
    };
    SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
}

pub(crate) fn click_button(button: u32, down: bool) {
    unsafe { inject_button(button, down) };
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let hook = HHOOK(KB_HOOK.load(Ordering::SeqCst) as *mut _);
    let passthrough = || unsafe { CallNextHookEx(hook, code, wparam, lparam) };
    if code < 0 {
        return passthrough();
    }
    let data = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    if is_own_injection(data.dwExtraInfo) {
        return passthrough();
    }
    let message = wparam.0 as u32;
    let down = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
    let up = matches!(message, WM_KEYUP | WM_SYSKEYUP);
    if !down && !up {
        return passthrough();
    }
    if down && data.vkCode == 0x75 && crate::autoclicker::is_running() {
        crate::autoclicker::stop();
    }
    SEEN_KEYS.fetch_add(1, Ordering::Relaxed);
    if GAME_MODE.load(Ordering::SeqCst) && matches!(data.vkCode, 0x5B | 0x5C) {
        SWALLOWED_KEYS.fetch_add(1, Ordering::Relaxed);
        if down {
            record("key", data.vkCode, None, "disable");
        }
        return LRESULT(1);
    }
    if !REMAP_ON.load(Ordering::SeqCst) {
        return passthrough();
    }
    let mapping = keys().lock().unwrap().get(&data.vkCode).copied();
    match mapping {
        Some(Some(target)) => {
            SWALLOWED_KEYS.fetch_add(1, Ordering::Relaxed);
            if down {
                record("key", data.vkCode, Some(target), "remap");
            }
            inject_key(target, down);
            LRESULT(1)
        }
        Some(None) => {
            SWALLOWED_KEYS.fetch_add(1, Ordering::Relaxed);
            if down {
                record("key", data.vkCode, None, "disable");
            }
            LRESULT(1)
        }
        None => passthrough(),
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let hook = HHOOK(MOUSE_HOOK.load(Ordering::SeqCst) as *mut _);
    let passthrough = || unsafe { CallNextHookEx(hook, code, wparam, lparam) };
    if code < 0 {
        return passthrough();
    }
    let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
    if is_own_injection(data.dwExtraInfo) {
        return passthrough();
    }
    let message = wparam.0 as u32;
    let (button, down) = match message {
        WM_LBUTTONDOWN => (0u32, true),
        WM_LBUTTONUP => (0, false),
        WM_RBUTTONDOWN => (1, true),
        WM_RBUTTONUP => (1, false),
        WM_MBUTTONDOWN => (2, true),
        WM_MBUTTONUP => (2, false),
        WM_XBUTTONDOWN => (x_button(data.mouseData), true),
        WM_XBUTTONUP => (x_button(data.mouseData), false),
        _ => return passthrough(),
    };
    SEEN_BUTTONS.fetch_add(1, Ordering::Relaxed);
    if !REMAP_ON.load(Ordering::SeqCst) {
        return passthrough();
    }
    let mapping = buttons().lock().unwrap().get(&button).cloned();
    let Some((action, target)) = mapping else {
        return passthrough();
    };
    let swallowed = match action.as_str() {
        "disable" => true,
        "button" => {
            inject_button(target, down);
            true
        }
        "key" => {
            inject_key(target, down);
            true
        }
        _ => false,
    };
    if swallowed {
        SWALLOWED_BUTTONS.fetch_add(1, Ordering::Relaxed);
        if down {
            let to = if action == "disable" {
                None
            } else {
                Some(target)
            };
            record("button", button, to, &action);
        }
        LRESULT(1)
    } else {
        passthrough()
    }
}

pub fn counters() -> (u64, u64, u64, u64) {
    (
        SEEN_KEYS.load(Ordering::Relaxed).max(0) as u64,
        SWALLOWED_KEYS.load(Ordering::Relaxed).max(0) as u64,
        SEEN_BUTTONS.load(Ordering::Relaxed).max(0) as u64,
        SWALLOWED_BUTTONS.load(Ordering::Relaxed).max(0) as u64,
    )
}

fn x_button(mouse_data: u32) -> u32 {
    if mouse_data >> 16 == 1 {
        3
    } else {
        4
    }
}

fn device_name(handle: usize) -> String {
    if let Some(cached) = names().lock().unwrap().get(&handle) {
        return cached.clone();
    }
    let name = unsafe {
        let mut size: u32 = 0;
        GetRawInputDeviceInfoW(HANDLE(handle as *mut _), RIDI_DEVICENAME, None, &mut size);
        if size == 0 || size > 1024 {
            String::new()
        } else {
            let mut buffer = vec![0u16; size as usize + 1];
            let read = GetRawInputDeviceInfoW(
                HANDLE(handle as *mut _),
                RIDI_DEVICENAME,
                Some(buffer.as_mut_ptr() as *mut c_void),
                &mut size,
            );
            if read == 0 {
                String::new()
            } else {
                String::from_utf16_lossy(&buffer[..read as usize])
                    .trim_end_matches('\0')
                    .to_string()
            }
        }
    };
    names().lock().unwrap().insert(handle, name.clone());
    name
}

fn note_report(name: &str, kind: &str) {
    if name.is_empty() {
        return;
    }
    let at = stamp();
    let mut table = reports().lock().unwrap();
    let entry = table
        .entry(name.to_ascii_lowercase())
        .or_insert_with(|| (kind.to_string(), VecDeque::new()));
    entry.1.push_back(at);
    while entry.1.len() > 4000 {
        entry.1.pop_front();
    }
}

unsafe fn handle_raw_input(lparam: LPARAM) {
    let header = std::mem::size_of::<RAWINPUTHEADER>() as u32;
    let handle = HRAWINPUT(lparam.0 as *mut _);
    let mut size: u32 = 0;
    GetRawInputData(handle, RID_INPUT, None, &mut size, header);
    if size == 0 || size > 64 * 1024 {
        return;
    }
    let mut buffer = raw_buf().lock().unwrap();
    if buffer.len() < size as usize {
        buffer.resize(size as usize, 0);
    }
    let written = GetRawInputData(
        handle,
        RID_INPUT,
        Some(buffer.as_mut_ptr() as *mut c_void),
        &mut size,
        header,
    );
    if written == 0 {
        return;
    }
    let raw = &*(buffer.as_ptr() as *const RAWINPUT);
    let kind = match raw.header.dwType {
        t if t == RIM_TYPEMOUSE.0 => "mouse",
        t if t == RIM_TYPEKEYBOARD.0 => "keyboard",
        t if t == RIM_TYPEHID.0 => "hid",
        _ => return,
    };
    let device = raw.header.hDevice.0 as usize;
    let key = if kind == "keyboard" {
        let kb = &raw.data.keyboard;
        Some((
            kb.VKey as u32,
            kb.MakeCode as u32,
            kb.Flags & 0x02 != 0,
            kb.Flags & 0x01 == 0,
        ))
    } else {
        None
    };
    drop(buffer);
    let name = device_name(device);
    note_report(&name, kind);
    if let Some((vk, scan, extended, down)) = key {
        if vk != 0 && vk != 0xFF {
            publish_live(&name, vk, scan, extended, down);
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_INPUT {
        handle_raw_input(lparam);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

fn panic_escape() {
    let held = unsafe {
        GetAsyncKeyState(VK_RCONTROL.0 as i32) < 0 && GetAsyncKeyState(VK_RSHIFT.0 as i32) < 0
    };
    if !held {
        *panic_since().lock().unwrap() = None;
        return;
    }
    let start = {
        let mut guard = panic_since().lock().unwrap();
        *guard.get_or_insert_with(Instant::now)
    };
    if start.elapsed() >= PANIC_HOLD && REMAP_ON.load(Ordering::SeqCst) {
        REMAP_ON.store(false, Ordering::SeqCst);
        crate::store::log_line("input remap disabled by panic hold (Right Ctrl + Right Shift 3s)");
        if let Some(app) = APP.get() {
            let _ = app.emit("input:changed", ());
        }
        *panic_since().lock().unwrap() = None;
    }
}

fn engine_thread() {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None).unwrap_or_default().into();
        let class_name: Vec<u16> = WINDOW_CLASS
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        if RegisterClassExW(&wc) == 0 {
            crate::store::log_line("input engine: raw input window class rejected");
            return;
        }
        let Ok(hwnd) = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            instance,
            None,
        ) else {
            crate::store::log_line("input engine: raw input window creation failed");
            return;
        };
        let devices = [
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x06,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            },
            RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x02,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            },
        ];
        if RegisterRawInputDevices(&devices, std::mem::size_of::<RAWINPUTDEVICE>() as u32).is_err()
        {
            crate::store::log_line("input engine: raw input registration rejected");
        }
        let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), instance, 0);
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), instance, 0);
        if let Ok(hook) = &keyboard {
            KB_HOOK.store(hook.0 as isize, Ordering::SeqCst);
        }
        if let Ok(hook) = &mouse {
            MOUSE_HOOK.store(hook.0 as isize, Ordering::SeqCst);
        }
        match (keyboard.is_ok(), mouse.is_ok()) {
            (true, true) => crate::store::log_line(
                "input engine started: keyboard + mouse hooks, raw input sink",
            ),
            (true, false) => crate::store::log_line(
                "input engine started: keyboard hook only (mouse hook rejected)",
            ),
            (false, true) => crate::store::log_line(
                "input engine started: mouse hook only (keyboard hook rejected)",
            ),
            (false, false) => {
                crate::store::log_line(
                    "input engine: both low level hooks rejected, raw input only",
                );
            }
        }
        ENGINE_ACTIVE.store(true, Ordering::SeqCst);
        let mut message = MSG::default();
        let mut last_check = Instant::now();
        loop {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if last_check.elapsed() >= Duration::from_millis(200) {
                last_check = Instant::now();
                panic_escape();
            }
            thread::sleep(Duration::from_millis(4));
        }
    }
}

pub fn start(app: AppHandle) {
    let _ = APP.set(app);
    thread::spawn(engine_thread);
}

pub fn parse_vid_pid(name: &str) -> (Option<u32>, Option<u32>) {
    let lower = name.to_ascii_lowercase();
    let grab = |marker: &str| -> Option<u32> {
        let index = lower.find(marker)?;
        let hex: String = lower[index + marker.len()..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .take(4)
            .collect();
        if hex.len() == 4 {
            u32::from_str_radix(&hex, 16).ok()
        } else {
            None
        }
    };
    (grab("vid_"), grab("pid_"))
}

fn short_path(name: &str) -> String {
    let clean = name.trim_end_matches('\0');
    let body = clean.split('#').nth(1).unwrap_or(clean);
    body.split('&')
        .map(|part| part.to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join(" · ")
}

pub fn stats(window_ms: u64) -> Vec<PollingStat> {
    let table = reports().lock().unwrap();
    let window = window_ms.clamp(200, 3000);
    let cutoff = stamp().saturating_sub(window);
    let mut out = Vec::new();
    for (device, (kind, stamps)) in table.iter() {
        let samples: Vec<u64> = stamps.iter().copied().filter(|at| *at >= cutoff).collect();
        if samples.len() < 4 {
            continue;
        }
        let first = samples[0];
        let last = samples[samples.len() - 1];
        let elapsed = last.saturating_sub(first).max(1) as f64;
        let rate = (samples.len() - 1) as f64 * 1000.0 / elapsed;
        let (vid, pid) = parse_vid_pid(device);
        out.push(PollingStat {
            device: short_path(device),
            transport: device.clone(),
            kind: kind.clone(),
            vid,
            pid,
            rate,
            reports: samples.len() as u32,
            window_ms: window,
        });
    }
    out.sort_by(|a, b| b.rate.total_cmp(&a.rate));
    out
}

pub fn reset_stats() {
    reports().lock().unwrap().clear();
}

fn spi_get(action: SYSTEM_PARAMETERS_INFO_ACTION, value: &mut u32) {
    unsafe {
        let _ = SystemParametersInfoW(
            action,
            0,
            Some(value as *mut u32 as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
}

pub fn system_get() -> MouseSystem {
    let mut speed = 10u32;
    let mut wheel = 3u32;
    let mut params = [0i32; 3];
    spi_get(SPI_GETMOUSESPEED, &mut speed);
    spi_get(SPI_GETWHEELSCROLLLINES, &mut wheel);
    let swap = unsafe { GetSystemMetrics(SM_SWAPBUTTON) };
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETMOUSE,
            0,
            Some(params.as_mut_ptr() as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    MouseSystem {
        pointer_speed: speed.clamp(1, 20),
        acceleration: params[2] != 0,
        wheel_lines: wheel.min(100),
        swap_buttons: swap != 0,
    }
}

pub fn system_set(next: &MouseSystem) -> MouseSystem {
    let speed = next.pointer_speed.clamp(1, 20);
    let wheel = next.wheel_lines.clamp(1, 100);
    let swap: u32 = if next.swap_buttons { 1 } else { 0 };
    let mut params = [0i32; 3];
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETMOUSE,
            0,
            Some(params.as_mut_ptr() as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        params[2] = if next.acceleration { 6 } else { 0 };
    }
    // wheel lines and swap travel through uiParam; mouse speed needs its own path
    spi_uint(SPI_SETWHEELSCROLLLINES, wheel);
    let applied = if spi_uint(SPI_SETMOUSESPEED, speed) {
        "spi"
    } else if persist_pointer_speed(speed) {
        "registry"
    } else {
        "rejected"
    };
    crate::store::log_line(&format!("pointer speed {} -> {}", speed, applied));
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_SETMOUSE,
            0,
            Some(params.as_mut_ptr() as *mut c_void),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        );
        let _ = SystemParametersInfoW(
            SPI_SETMOUSEBUTTONSWAP,
            swap,
            None,
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        );
    }
    system_get()
}

fn spi_uint(action: SYSTEM_PARAMETERS_INFO_ACTION, value: u32) -> bool {
    unsafe {
        SystemParametersInfoW(action, value, None, SPIF_UPDATEINIFILE | SPIF_SENDCHANGE).is_ok()
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Some Windows builds refuse SPI_SETMOUSESPEED, so the speed is written to the
/// Control Panel key and announced with WM_SETTINGCHANGE instead.
fn persist_pointer_speed(speed: u32) -> bool {
    let section = wide("Control Panel\\Mouse");
    let name = wide("MouseSensitivity");
    let text = wide(&speed.to_string());
    unsafe {
        let mut key = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(section.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut key,
        )
        .is_err()
        {
            return false;
        }
        let mut bytes = Vec::with_capacity(text.len() * 2);
        for unit in &text {
            bytes.extend_from_slice(&unit.to_ne_bytes());
        }
        let written = RegSetValueExW(key, PCWSTR(name.as_ptr()), 0, REG_SZ, Some(&bytes));
        let _ = RegCloseKey(key);
        if written.is_err() {
            return false;
        }
        let mut result = 0usize;
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(name.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            Some(&mut result),
        );
        true
    }
}

fn key_name(vk: u32) -> Option<String> {
    let mut buffer = [0u16; 64];
    let scan = unsafe { MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) };
    if scan == 0 && !matches!(vk, 0x08 | 0x0D | 0x1B | 0x20 | 0x5B | 0x5C | 0x5D) {
        return None;
    }
    let param = ((scan << 16) | (if is_extended(vk) { 1 << 24 } else { 0 })) as i32;
    let len = unsafe { GetKeyNameTextW(param, &mut buffer) };
    if len <= 0 {
        return None;
    }
    let name: String = String::from_utf16_lossy(&buffer[..len as usize])
        .trim()
        .to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

pub fn key_names() -> Vec<(u32, String)> {
    let mut names: Vec<(u32, String)> = (0x08..0x100u32)
        .filter(|vk| !matches!(vk, 0x09 | 0x0A | 0x0B | 0x0C | 0x1A | 0x28..=0x2F | 0x3A..=0x40 | 0x5E | 0x5F))
        .filter_map(|vk| key_name(vk).map(|name| (vk, name)))
        .collect();
    // F13..F24 carry no scan code on ordinary layouts, so GetKeyNameTextW cannot name them
    for vk in 0x7C..=0x87u32 {
        if names.iter().any(|(known, _)| *known == vk) {
            continue;
        }
        names.push((vk, format!("F{}", vk - 0x7C + 13)));
    }
    names.sort_by_key(|(vk, _)| *vk);
    names
}

pub fn sanitize(map: &InputMap) -> InputMap {
    InputMap {
        enabled: map.enabled,
        keys: map
            .keys
            .iter()
            .filter(|entry| entry.vk != 0)
            .filter(|entry| match entry.target {
                Some(target) => target != 0 && target != entry.vk,
                None => true,
            })
            .cloned()
            .collect(),
        buttons: map
            .buttons
            .iter()
            .filter(|entry| entry.button < 5)
            .filter(|entry| entry.target < 5 || entry.action == "disable")
            .filter(|entry| matches!(entry.action.as_str(), "disable" | "button" | "key"))
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_hid_paths_to_stable_keys() {
        let a = path_key(r"\\?\HID#VID_342D&PID_E411&MI_02&Col02#7&1&0001#{4d1e55b2}");
        let b = path_key("hid#vid_342d&pid_e411&mi_02&col02#7&1&0001");
        assert_eq!(a, b);
        assert!(!a.contains("col02"));
        assert!(!a.contains('{'));
    }

    #[test]
    fn extracts_vid_and_pid_from_paths() {
        assert_eq!(
            parse_vid_pid(r"HID\VID_7768&PID_6426&MI_01&Col02"),
            (Some(0x7768), Some(0x6426))
        );
        assert_eq!(parse_vid_pid("keyboard"), (None, None));
    }
}
