use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::input::click_button;

const TICK: Duration = Duration::from_millis(8);

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoConfig {
    pub button: String,
    pub mode: String,
    pub interval_ms: u64,
    pub jitter_ms: u64,
    pub limit: u64,
    pub duration_ms: u64,
    pub delay_ms: u64,
    pub hold_ms: u64,
}

impl AutoConfig {
    fn validate(&self) -> Result<(), String> {
        if !matches!(self.button.as_str(), "left" | "right" | "middle") {
            return Err("unknown mouse button".into());
        }
        if !matches!(self.mode.as_str(), "single" | "double" | "hold") {
            return Err("unknown click mode".into());
        }
        if self.interval_ms < 1 || self.interval_ms > 60_000 {
            return Err("interval must be 1..60000 ms".into());
        }
        if self.jitter_ms > self.interval_ms {
            return Err("jitter cannot exceed the interval".into());
        }
        if self.limit > 10_000_000 {
            return Err("click limit is too large".into());
        }
        if self.duration_ms > 24 * 60 * 60 * 1000 {
            return Err("duration must be under 24 hours".into());
        }
        if self.delay_ms > 60_000 {
            return Err("start delay must be under 60 seconds".into());
        }
        if self.hold_ms > 10_000 {
            return Err("hold time must be under 10 seconds".into());
        }
        Ok(())
    }

    fn button_index(&self) -> u32 {
        match self.button.as_str() {
            "right" => 1,
            "middle" => 2,
            _ => 0,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AutoStatus {
    pub running: bool,
    pub clicks: u64,
    pub elapsed_ms: u64,
    pub config: Option<AutoConfig>,
}

static RUNNING: AtomicBool = AtomicBool::new(false);
static RUN_ID: AtomicU64 = AtomicU64::new(0);
static CLICKS: AtomicU64 = AtomicU64::new(0);
static ELAPSED_MS: AtomicU64 = AtomicU64::new(0);
static STARTED: Mutex<Option<Instant>> = Mutex::new(None);
static CONFIG: Mutex<Option<AutoConfig>> = Mutex::new(None);
static APP: OnceLock<Mutex<Option<AppHandle>>> = OnceLock::new();
static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

fn app_slot() -> &'static Mutex<Option<AppHandle>> {
    APP.get_or_init(|| Mutex::new(None))
}

fn freeze_elapsed() {
    if let Some(started) = *STARTED.lock().unwrap() {
        let elapsed = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        ELAPSED_MS.store(elapsed, Ordering::SeqCst);
    }
}

pub fn status() -> AutoStatus {
    let running = RUNNING.load(Ordering::SeqCst);
    let elapsed_ms = if running {
        STARTED
            .lock()
            .unwrap()
            .map(|started| started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64)
            .unwrap_or(0)
    } else {
        ELAPSED_MS.load(Ordering::SeqCst)
    };
    AutoStatus {
        running,
        clicks: CLICKS.load(Ordering::Relaxed),
        elapsed_ms,
        config: CONFIG.lock().unwrap().clone(),
    }
}

fn emit(app: &AppHandle) {
    let _ = app.emit("autoclicker:state", status());
}

pub fn is_running() -> bool {
    RUNNING.load(Ordering::SeqCst)
}

fn emit_current() {
    if let Some(app) = app_slot().lock().unwrap().clone() {
        emit(&app);
    }
}

pub fn stop() -> AutoStatus {
    if RUNNING.swap(false, Ordering::SeqCst) {
        RUN_ID.fetch_add(1, Ordering::SeqCst);
        freeze_elapsed();
        crate::store::log_line("auto clicker stopped");
        emit_current();
    }
    status()
}

pub fn start(app: AppHandle, config: AutoConfig) -> Result<AutoStatus, String> {
    config.validate()?;
    if RUNNING.load(Ordering::SeqCst) {
        return Err("auto clicker is already running".into());
    }
    *app_slot().lock().unwrap() = Some(app.clone());
    *CONFIG.lock().unwrap() = Some(config.clone());
    CLICKS.store(0, Ordering::SeqCst);
    ELAPSED_MS.store(0, Ordering::SeqCst);
    *STARTED.lock().unwrap() = None;
    let id = RUN_ID.fetch_add(1, Ordering::SeqCst) + 1;
    RUNNING.store(true, Ordering::SeqCst);
    crate::store::log_line(&format!(
        "auto clicker started: {} {} every {} ms",
        config.mode, config.button, config.interval_ms
    ));
    emit(&app);
    worker(app, config, id);
    Ok(status())
}

fn jitter(base: u64, spread: u64) -> u64 {
    if spread == 0 {
        return base;
    }
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    let span = spread.saturating_mul(2).saturating_add(1);
    base.saturating_sub(spread).saturating_add(x % span)
}

fn sleep_checked(ms: u64, id: u64) -> bool {
    let deadline = Instant::now() + Duration::from_millis(ms);
    while RUNNING.load(Ordering::SeqCst) && RUN_ID.load(Ordering::SeqCst) == id {
        let now = Instant::now();
        if now >= deadline {
            return true;
        }
        thread::sleep(TICK.min(deadline - now));
    }
    false
}

fn worker(app: AppHandle, config: AutoConfig, id: u64) {
    thread::spawn(move || {
        let started = Instant::now();
        *STARTED.lock().unwrap() = Some(started);
        emit(&app);

        if !sleep_checked(config.delay_ms, id) {
            finish(&app);
            return;
        }

        let button = config.button_index();
        let mut clicks = 0u64;
        let mut last_emit = Instant::now();

        while RUNNING.load(Ordering::SeqCst) && RUN_ID.load(Ordering::SeqCst) == id {
            if config.limit > 0 && clicks >= config.limit {
                break;
            }
            if config.duration_ms > 0
                && started.elapsed() >= Duration::from_millis(config.duration_ms + config.delay_ms)
            {
                break;
            }

            click_button(button, true);
            let held = if config.mode == "hold" {
                sleep_checked(config.hold_ms.max(1), id)
            } else {
                thread::sleep(Duration::from_millis(12));
                RUNNING.load(Ordering::SeqCst) && RUN_ID.load(Ordering::SeqCst) == id
            };
            click_button(button, false);
            if !held {
                break;
            }

            let mut presses = 1u64;
            if config.mode == "double" {
                thread::sleep(Duration::from_millis(45));
                click_button(button, true);
                thread::sleep(Duration::from_millis(12));
                click_button(button, false);
                presses = 2;
            }

            clicks = clicks.saturating_add(presses);
            CLICKS.store(clicks, Ordering::SeqCst);

            if last_emit.elapsed() >= Duration::from_millis(120) {
                emit(&app);
                last_emit = Instant::now();
            }

            if !sleep_checked(jitter(config.interval_ms, config.jitter_ms), id) {
                break;
            }
        }

        finish(&app);
    });
}

fn finish(app: &AppHandle) {
    let was_running = RUNNING.swap(false, Ordering::SeqCst);
    RUN_ID.fetch_add(1, Ordering::SeqCst);
    freeze_elapsed();
    if was_running {
        crate::store::log_line(&format!(
            "auto clicker finished: {} clicks",
            CLICKS.load(Ordering::SeqCst)
        ));
    }
    emit(app);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AutoConfig {
        AutoConfig {
            button: "left".into(),
            mode: "single".into(),
            interval_ms: 100,
            jitter_ms: 10,
            limit: 0,
            duration_ms: 0,
            delay_ms: 0,
            hold_ms: 60,
        }
    }

    #[test]
    fn accepts_valid_config() {
        assert!(sample().validate().is_ok());
    }

    #[test]
    fn rejects_unknown_button() {
        let mut cfg = sample();
        cfg.button = "side".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_zero_interval() {
        let mut cfg = sample();
        cfg.interval_ms = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_jitter_wider_than_interval() {
        let mut cfg = sample();
        cfg.jitter_ms = 200;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_oversized_start_delay() {
        let mut cfg = sample();
        cfg.delay_ms = 90_000;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn jitter_stays_in_range() {
        for _ in 0..200 {
            let value = jitter(50, 20);
            assert!((30..=70).contains(&value), "jitter produced {value}");
        }
        assert_eq!(jitter(50, 0), 50);
    }

    #[test]
    fn button_index_matches_names() {
        let mut cfg = sample();
        assert_eq!(cfg.button_index(), 0);
        cfg.button = "right".into();
        assert_eq!(cfg.button_index(), 1);
        cfg.button = "middle".into();
        assert_eq!(cfg.button_index(), 2);
    }
}
