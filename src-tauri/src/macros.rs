use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::model::{now_ms, Macro, MacroEvent};

struct Recorder {
    events: Vec<MacroEvent>,
    started: Instant,
}

static APP: OnceLock<AppHandle> = OnceLock::new();
static RECORDER: Mutex<Option<Recorder>> = Mutex::new(None);
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
static KB_HOOK: AtomicIsize = AtomicIsize::new(0);
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);
static STOP_PLAYBACK: AtomicBool = AtomicBool::new(false);
static PLAYING: Mutex<Option<String>> = Mutex::new(None);
static HOTKEYS_TX: OnceLock<Mutex<Sender<HotkeyCmd>>> = OnceLock::new();

enum HotkeyCmd {
    Set {
        macro_id: String,
        hotkey: String,
        respond: Sender<bool>,
    },
    Reload(Vec<(String, String)>),
}

pub fn init(app: AppHandle) {
    let _ = APP.set(app.clone());
    let tx = spawn_hotkey_thread(app);
    let _ = HOTKEYS_TX.set(Mutex::new(tx));
}

#[derive(Serialize, Clone)]
struct MacroState {
    playing: Option<String>,
}

fn emit_state(playing: Option<String>) {
    if let Some(app) = APP.get() {
        let _ = app.emit("macro:state", MacroState { playing });
    }
}

fn record_event(event: MacroEvent) {
    let mut guard = RECORDER.lock().unwrap();
    let Some(rec) = guard.as_mut() else { return };
    let mut event = event;
    event.t = rec.started.elapsed().as_millis() as u64;
    rec.events.push(event.clone());
    drop(guard);
    if let Some(app) = APP.get() {
        let _ = app.emit("macro:event", event);
    }
}

fn key_label(vk: u32, scan: u32, extended: bool) -> String {
    let named = match vk {
        0x08 => Some("Backspace"),
        0x09 => Some("Tab"),
        0x0D => Some("Enter"),
        0x10 | 0xA0 => Some("Shift"),
        0x11 | 0xA2 => Some("Ctrl"),
        0x12 | 0xA4 => Some("Alt"),
        0x13 => Some("Pause"),
        0x14 => Some("CapsLock"),
        0x1B => Some("Esc"),
        0x20 => Some("Space"),
        0x21 => Some("PageUp"),
        0x22 => Some("PageDown"),
        0x23 => Some("End"),
        0x24 => Some("Home"),
        0x25 => Some("Left"),
        0x26 => Some("Up"),
        0x27 => Some("Right"),
        0x28 => Some("Down"),
        0x2C => Some("PrintScreen"),
        0x2D => Some("Insert"),
        0x2E => Some("Delete"),
        0x5B | 0x5C => Some("Win"),
        0x5D => Some("Menu"),
        0x90 => Some("NumLock"),
        0x91 => Some("ScrollLock"),
        _ => None,
    };
    if let Some(name) = named {
        return name.into();
    }
    if (0x70..=0x87).contains(&vk) {
        return format!("F{}", vk - 0x6F);
    }
    if (0x60..=0x69).contains(&vk) {
        return format!("Num{}", vk - 0x60);
    }
    if let Ok(ch) = char::try_from(vk) {
        if ch.is_ascii_graphic() {
            return ch.to_ascii_uppercase().to_string();
        }
    }
    let mut buf = [0u16; 64];
    let mut lparam = (scan & 0xFF) << 16;
    if extended {
        lparam |= 1 << 24;
    }
    let len = unsafe { GetKeyNameTextW(lparam as i32, &mut buf) };
    if len > 0 {
        return String::from_utf16_lossy(&buf[..len as usize]);
    }
    format!("VK {:02X}", vk)
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        if (info.flags.0 & LLKHF_INJECTED.0) == 0 {
            let msg = wparam.0 as u32;
            let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
            if down || up {
                let scan = unsafe { MapVirtualKeyW(info.vkCode, MAPVK_VK_TO_VSC) };
                let label = key_label(info.vkCode, scan, (info.flags.0 & LLKHF_EXTENDED.0) != 0);
                record_event(MacroEvent {
                    t: 0,
                    kind: "key".into(),
                    label,
                    down,
                    vk: info.vkCode,
                    x: None,
                    y: None,
                });
            }
        }
    }
    unsafe { CallNextHookEx(HHOOK::default(), code, wparam, lparam) }
}

fn wheel_label(data: u32) -> (&'static str, bool) {
    let delta = (data >> 16) as i16;
    if delta > 0 {
        ("Wheel up", true)
    } else {
        ("Wheel down", true)
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
        if (info.flags & LLMHF_INJECTED) == 0 {
            let msg = wparam.0 as u32;
            let mapped: Option<(&str, bool)> = match msg {
                WM_LBUTTONDOWN => Some(("LMB", true)),
                WM_LBUTTONUP => Some(("LMB", false)),
                WM_RBUTTONDOWN => Some(("RMB", true)),
                WM_RBUTTONUP => Some(("RMB", false)),
                WM_MBUTTONDOWN => Some(("MMB", true)),
                WM_MBUTTONUP => Some(("MMB", false)),
                WM_XBUTTONDOWN => Some(("XMB", true)),
                WM_XBUTTONUP => Some(("XMB", false)),
                WM_MOUSEWHEEL => Some(wheel_label(info.mouseData)),
                _ => None,
            };
            if let Some((label, down)) = mapped {
                record_event(MacroEvent {
                    t: 0,
                    kind: "mouse".into(),
                    label: label.into(),
                    down,
                    vk: 0,
                    x: Some(info.pt.x),
                    y: Some(info.pt.y),
                });
            }
        }
    }
    unsafe { CallNextHookEx(HHOOK::default(), code, wparam, lparam) }
}

fn hook_thread() {
    unsafe {
        let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), HINSTANCE::default(), 0);
        let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), HINSTANCE::default(), 0);
        match (kb, ms) {
            (Ok(kb), Ok(ms)) => {
                KB_HOOK.store(kb.0 as isize, Ordering::SeqCst);
                MOUSE_HOOK.store(ms.0 as isize, Ordering::SeqCst);
                HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
            }
            (Ok(kb), Err(_)) => {
                let _ = UnhookWindowsHookEx(kb);
                return;
            }
            (Err(_), Ok(ms)) => {
                let _ = UnhookWindowsHookEx(ms);
                return;
            }
            (Err(_), Err(_)) => return,
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            if msg.message == WM_QUIT {
                break;
            }
        }
        let kb = KB_HOOK.swap(0, Ordering::SeqCst);
        let ms = MOUSE_HOOK.swap(0, Ordering::SeqCst);
        if kb != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(kb as *mut _));
        }
        if ms != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(ms as *mut _));
        }
    }
}

pub fn start_recording() -> Result<(), String> {
    let mut guard = RECORDER.lock().unwrap();
    if guard.is_some() {
        return Err("recording already running".into());
    }
    *guard = Some(Recorder {
        events: Vec::new(),
        started: Instant::now(),
    });
    drop(guard);
    std::thread::spawn(hook_thread);
    std::thread::sleep(Duration::from_millis(60));
    if HOOK_THREAD.load(Ordering::SeqCst) == 0 {
        *RECORDER.lock().unwrap() = None;
        return Err("failed to install input hooks".into());
    }
    crate::store::log_line("macro recording started");
    Ok(())
}

fn stop_hooks() {
    let thread = HOOK_THREAD.load(Ordering::SeqCst);
    if thread != 0 {
        unsafe {
            let _ = PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn stop_recording() -> Result<Macro, String> {
    let taken = RECORDER.lock().unwrap().take();
    stop_hooks();
    let Some(rec) = taken else {
        return Err("recording is not running".into());
    };
    if rec.events.is_empty() {
        return Err("no input events were recorded".into());
    }
    crate::store::log_line(&format!(
        "macro recording stopped: {} events",
        rec.events.len()
    ));
    Ok(Macro {
        id: format!("macro-{}", now_ms()),
        name: String::new(),
        created_at: now_ms(),
        events: rec.events,
        repeat: 1,
        loop_enabled: false,
        hotkey: None,
    })
}

pub fn cancel_recording() {
    *RECORDER.lock().unwrap() = None;
    stop_hooks();
}

fn is_extended(vk: u32) -> bool {
    matches!(
        vk,
        0x21 | 0x22 | 0x23 | 0x24 | 0x25 | 0x26 | 0x27 | 0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x5D
    )
}

fn send_key(vk: u32, down: bool) {
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
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: crate::input::MACRO_MARK,
            },
        },
    };
    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

fn mouse_flag(label: &str, down: bool) -> Option<MOUSE_EVENT_FLAGS> {
    match (label, down) {
        ("LMB", true) => Some(MOUSEEVENTF_LEFTDOWN),
        ("LMB", false) => Some(MOUSEEVENTF_LEFTUP),
        ("RMB", true) => Some(MOUSEEVENTF_RIGHTDOWN),
        ("RMB", false) => Some(MOUSEEVENTF_RIGHTUP),
        ("MMB", true) => Some(MOUSEEVENTF_MIDDLEDOWN),
        ("MMB", false) => Some(MOUSEEVENTF_MIDDLEUP),
        _ => None,
    }
}

fn send_mouse(event: &MacroEvent) {
    if event.label.starts_with("Wheel") {
        let delta: u32 = if event.label.contains("up") {
            120
        } else {
            (-120i32) as u32
        };
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: delta,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: crate::input::MACRO_MARK,
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        return;
    }
    if let (Some(x), Some(y)) = (event.x, event.y) {
        unsafe {
            let _ = SetCursorPos(x, y);
        }
    }
    if let Some(flag) = mouse_flag(&event.label, event.down) {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flag,
                    time: 0,
                    dwExtraInfo: crate::input::MACRO_MARK,
                },
            },
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }
}

fn playback_thread(mac: Macro) {
    let macro_id = mac.id.clone();
    STOP_PLAYBACK.store(false, Ordering::SeqCst);
    *PLAYING.lock().unwrap() = Some(macro_id.clone());
    emit_state(Some(macro_id.clone()));
    crate::store::log_line(&format!("macro playback: {}", mac.name));
    let mut rounds = 0u32;
    loop {
        if STOP_PLAYBACK.load(Ordering::SeqCst) {
            break;
        }
        let mut elapsed = 0u64;
        for event in &mac.events {
            if STOP_PLAYBACK.load(Ordering::SeqCst) {
                break;
            }
            let wait = event.t.saturating_sub(elapsed);
            if wait > 0 {
                std::thread::sleep(Duration::from_millis(wait.min(5000)));
            }
            elapsed = event.t;
            match event.kind.as_str() {
                "key" => send_key(event.vk, event.down),
                "mouse" => send_mouse(event),
                _ => {}
            }
        }
        rounds += 1;
        if !mac.loop_enabled && rounds >= mac.repeat.max(1) {
            break;
        }
    }
    *PLAYING.lock().unwrap() = None;
    emit_state(None);
}

pub fn play(mac: Macro) -> Result<(), String> {
    if PLAYING.lock().unwrap().is_some() {
        return Err("macro playback already running".into());
    }
    std::thread::spawn(move || playback_thread(mac));
    Ok(())
}

pub fn stop_playback() {
    STOP_PLAYBACK.store(true, Ordering::SeqCst);
}

fn parse_hotkey(hotkey: &str) -> Option<(HOT_KEY_MODIFIERS, u32)> {
    let mut mods = MOD_NOREPEAT;
    let mut key: Option<u32> = None;
    for raw in hotkey.split('+') {
        let token = raw.trim();
        if token.is_empty() {
            return None;
        }
        let lower = token.to_ascii_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "shift" => mods |= MOD_SHIFT,
            "alt" => mods |= MOD_ALT,
            "win" | "meta" | "super" => mods |= MOD_WIN,
            _ => {
                if key.is_some() {
                    return None;
                }
                if lower.len() >= 2 && lower.starts_with('f') {
                    if let Ok(n) = lower[1..].parse::<u32>() {
                        if (1..=24).contains(&n) {
                            key = Some(0x6F + n);
                            continue;
                        }
                    }
                }
                let special = match lower.as_str() {
                    "space" => Some(0x20),
                    "enter" | "return" => Some(0x0D),
                    "esc" | "escape" => Some(0x1B),
                    "tab" => Some(0x09),
                    "backspace" => Some(0x08),
                    "insert" => Some(0x2D),
                    "delete" | "del" => Some(0x2E),
                    "home" => Some(0x24),
                    "end" => Some(0x23),
                    "pageup" => Some(0x21),
                    "pagedown" => Some(0x22),
                    "up" => Some(0x26),
                    "down" => Some(0x28),
                    "left" => Some(0x25),
                    "right" => Some(0x27),
                    _ => None,
                };
                if let Some(vk) = special {
                    key = Some(vk);
                } else if token.chars().count() == 1 {
                    let ch = token.chars().next().unwrap() as u16;
                    let scan = unsafe { VkKeyScanW(ch) };
                    if scan == -1 {
                        return None;
                    }
                    key = Some((scan & 0xFF) as u32);
                } else {
                    return None;
                }
            }
        }
    }
    Some((mods, key?))
}

fn macros_snapshot(app: &AppHandle) -> Vec<Macro> {
    use tauri::Manager;
    app.state::<crate::store::AppState>()
        .macros
        .lock()
        .map(|m| m.clone())
        .unwrap_or_default()
}

fn spawn_hotkey_thread(app: AppHandle) -> Sender<HotkeyCmd> {
    let (tx, rx) = channel::<HotkeyCmd>();
    std::thread::spawn(move || {
        let mut registered: Vec<(i32, String)> = Vec::new();
        let mut next_id = 0xA000i32;
        loop {
            let cmd = rx.recv_timeout(Duration::from_millis(40));
            match cmd {
                Ok(HotkeyCmd::Set {
                    macro_id,
                    hotkey,
                    respond,
                }) => {
                    if let Some((id, _)) = registered.iter().find(|(_, m)| *m == macro_id) {
                        unsafe {
                            let _ = UnregisterHotKey(HWND::default(), *id);
                        }
                    }
                    registered.retain(|(_, m)| *m != macro_id);
                    let ok = if let Some((mods, vk)) = parse_hotkey(&hotkey) {
                        next_id += 1;
                        let id = next_id;
                        let result = unsafe { RegisterHotKey(HWND::default(), id, mods, vk) };
                        if result.is_ok() {
                            registered.push((id, macro_id.clone()));
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    let _ = respond.send(ok);
                }
                Ok(HotkeyCmd::Reload(list)) => {
                    unsafe {
                        for (id, _) in registered.drain(..) {
                            let _ = UnregisterHotKey(HWND::default(), id);
                        }
                    }
                    for (macro_id, hotkey) in list {
                        if let Some((mods, vk)) = parse_hotkey(&hotkey) {
                            next_id += 1;
                            let id = next_id;
                            if unsafe { RegisterHotKey(HWND::default(), id, mods, vk) }.is_ok() {
                                registered.push((id, macro_id));
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
            let mut msg = MSG::default();
            unsafe {
                while PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_HOTKEY {
                        let id = msg.wParam.0 as i32;
                        if let Some((_, macro_id)) = registered.iter().find(|(hid, _)| *hid == id) {
                            if let Some(mac) = macros_snapshot(&app)
                                .into_iter()
                                .find(|m| m.id == *macro_id)
                            {
                                let _ = play(mac);
                            }
                        }
                    }
                }
            }
        }
        unsafe {
            for (id, _) in registered {
                let _ = UnregisterHotKey(HWND::default(), id);
            }
        }
    });
    tx
}

pub fn set_hotkey(macro_id: &str, hotkey: &str) -> bool {
    let Some(tx) = HOTKEYS_TX.get() else {
        return false;
    };
    let (respond, wait) = channel();
    let cmd = HotkeyCmd::Set {
        macro_id: macro_id.to_string(),
        hotkey: hotkey.to_string(),
        respond,
    };
    if tx.lock().unwrap().send(cmd).is_err() {
        return false;
    }
    wait.recv_timeout(Duration::from_secs(2)).unwrap_or(false)
}

pub fn reload_hotkeys(app: &AppHandle) {
    let Some(tx) = HOTKEYS_TX.get() else { return };
    let pairs = macros_snapshot(app)
        .into_iter()
        .filter_map(|m| m.hotkey.map(|h| (m.id, h)))
        .collect();
    let _ = tx.lock().unwrap().send(HotkeyCmd::Reload(pairs));
}
