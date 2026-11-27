//! Performance probe (GAP-CLOSURE-PLAN.md 16.8).
//!
//! Activated with `--perf-probe [idle|latency]`. It prints machine-readable
//! `starship_perf` lines that `xtask perf_budget` parses. It never runs in a
//! normal launch and changes no UI, layout or behavior.

use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProbeMode {
    Idle,
    Latency,
}

/// Parses `--perf-probe` / `--perf-probe=latency` / `--perf-probe latency`.
pub fn parse_mode() -> Option<ProbeMode> {
    let args: Vec<String> = std::env::args().collect();
    for (i, arg) in args.iter().enumerate() {
        if arg == "--perf-probe" {
            let value = args.get(i + 1).map_or("idle", String::as_str);
            return Some(if value == "latency" {
                ProbeMode::Latency
            } else {
                ProbeMode::Idle
            });
        }
        if let Some(value) = arg.strip_prefix("--perf-probe=") {
            return Some(if value == "latency" {
                ProbeMode::Latency
            } else {
                ProbeMode::Idle
            });
        }
    }
    None
}

pub fn spawn(app: AppHandle, mode: ProbeMode) {
    std::thread::spawn(move || {
        // Let the windows and webview settle before measuring.
        std::thread::sleep(Duration::from_secs(3));
        match mode {
            ProbeMode::Idle => {
                // "Idle core" = no UI on screen (minimized to tray), matching
                // the 0.2 Hz sampling mode in GAP-CLOSURE-PLAN.md 14.2. xtask
                // samples /proc after an 8 s warm-up; hold idle for 10 s so
                // the measured window is steady state.
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
                if let Some(w) = app.get_webview_window("hud") {
                    let _ = w.hide();
                }
                std::thread::sleep(Duration::from_secs(8));
                std::thread::sleep(Duration::from_secs(10));
                println!("starship_perf kind=done mode=idle");
            }
            ProbeMode::Latency => {
                measure_hud_toggle(&app);
                measure_event_dispatch(&app);
                println!("starship_perf kind=done mode=latency");
            }
        }
        app.exit(0);
    });
}

fn measure_hud_toggle(app: &AppHandle) {
    let Some(hud) = app.get_webview_window("hud") else {
        println!("starship_perf kind=error reason=hud_window_missing");
        return;
    };
    // Warm up once so the first measurement is not a cold window map.
    let _ = hud.show();
    let _ = hud.hide();
    std::thread::sleep(Duration::from_millis(250));

    for _ in 0..10 {
        let start = Instant::now();
        let visible = hud.is_visible().unwrap_or(false);
        let _ = if visible { hud.hide() } else { hud.show() };
        let _ = hud.set_focus();
        println!(
            "starship_perf kind=hud_toggle ms={:.3}",
            start.elapsed().as_secs_f64() * 1000.0
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn measure_event_dispatch(app: &AppHandle) {
    // 1 Hz health + 4 Hz progress for 5 seconds, matching the budget scenario.
    let start = Instant::now();
    let mut next_health = Duration::ZERO;
    let mut next_progress = Duration::ZERO;
    let horizon = Duration::from_secs(5);
    while start.elapsed() < horizon {
        let now = start.elapsed();
        if now >= next_health {
            let t = Instant::now();
            let _ = app.emit(
                "evt://health/tick",
                u64::try_from(now.as_millis()).unwrap_or(u64::MAX),
            );
            println!(
                "starship_perf kind=event_dispatch channel=health ms={:.3}",
                t.elapsed().as_secs_f64() * 1000.0
            );
            next_health += Duration::from_secs(1);
        }
        if now >= next_progress {
            let t = Instant::now();
            let _ = app.emit(
                "evt://mission/progress",
                u64::try_from(now.as_millis()).unwrap_or(u64::MAX),
            );
            println!(
                "starship_perf kind=event_dispatch channel=progress ms={:.3}",
                t.elapsed().as_secs_f64() * 1000.0
            );
            next_progress += Duration::from_millis(250);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
