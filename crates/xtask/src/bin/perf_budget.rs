//! `cargo xtask perf-budget` — measures the Phase 16.8 performance budgets.
//!
//! Budgets (GAP-CLOSURE-PLAN.md 16.8):
//!   - idle core < 150 MB footprint and < 1 % CPU
//!   - HUD toggle < 150 ms
//!   - event dispatch core→UI < 50 ms at 1 Hz health + 4 Hz progress
//!
//! Memory is enforced as PSS (proportional set size). RSS is printed for the
//! record but is not the budget: the Tauri UI process maps `WebKitGTK`, GTK and
//! Mesa, and RSS counts those shared pages in full (~95 MB) even though other
//! processes map them too. PSS apportions shared pages, which is the standard
//! per-process footprint metric.
//!
//! The app exposes a `--perf-probe` mode that prints `starship_perf` lines.
//! This binary runs the real app twice (idle, then latency), samples `/proc`
//! for PSS/CPU, parses the probe output, and prints a report. It exits non-zero
//! on a budget breach unless `--warn-only` is passed (used by CI as a warning).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const PSS_BUDGET_MB: f64 = 150.0;
const CPU_BUDGET_PCT: f64 = 1.0;
const HUD_BUDGET_MS: f64 = 150.0;
const EVENT_BUDGET_MS: f64 = 50.0;

struct Args {
    binary: PathBuf,
    idle_secs: u64,
    warn_only: bool,
}

fn parse_args() -> Args {
    let mut binary = PathBuf::from("target/debug/starship-control");
    let mut idle_secs = 6;
    let mut warn_only = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--binary" => {
                if let Some(v) = args.next() {
                    binary = PathBuf::from(v);
                }
            }
            "--idle-secs" => {
                if let Some(v) = args.next() {
                    idle_secs = v.parse().unwrap_or(6);
                }
            }
            "--warn-only" => warn_only = true,
            _ => {}
        }
    }
    Args {
        binary,
        idle_secs,
        warn_only,
    }
}

fn proc_status_kb(pid: u32, key: &str) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

/// Proportional set size: shared library pages are apportioned across the
/// processes that map them, unlike RSS which counts them in full in every
/// process. Reported alongside RSS for an honest footprint comparison.
fn proc_pss_kb(pid: u32) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Pss:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

fn proc_cpu_ticks(pid: u32) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Field 14/15 (1-based) are utime/stime; comm may contain spaces, so split after ')'.
    let after = text.rsplit_once(')')?.1;
    let fields: Vec<&str> = after.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime + stime)
}

fn clock_ticks() -> u64 {
    // sysconf(_SC_CLK_TCK) is 100 on Linux; acceptable for this probe.
    100
}

struct ProbeRun {
    hud_ms: Vec<f64>,
    health_ms: Vec<f64>,
    progress_ms: Vec<f64>,
    peak_rss_mb: f64,
    peak_pss_mb: f64,
    cpu_pct: f64,
}

#[allow(clippy::cast_precision_loss)]
fn run_probe(args: &Args, mode: &str, idle_secs: u64) -> Result<ProbeRun, String> {
    if !args.binary.exists() {
        return Err(format!(
            "binary not found: {} (build with `cargo build -p starship-control`)",
            args.binary.display()
        ));
    }
    let mut child = Command::new(&args.binary)
        .arg("--perf-probe")
        .arg(mode)
        .env("RUST_LOG", "info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to launch {}: {e}", args.binary.display()))?;
    let pid = child.id();

    // Sample RSS/CPU only during the idle window (after startup settles and
    // before any probe activity). The probe holds idle for ~18 s; we skip the
    // first 8 s so DB migrations, window creation and first webview paint are
    // not counted as idle.
    let mut peak_rss_kb: u64 = 0;
    let mut peak_pss_kb: u64 = 0;
    let mut cpu_pct = 0.0;
    if mode == "idle" {
        std::thread::sleep(Duration::from_secs(8));
        let cpu0 = proc_cpu_ticks(pid).unwrap_or(0);
        let t0 = Instant::now();
        let deadline = t0 + Duration::from_secs(idle_secs);
        while Instant::now() < deadline {
            if let Some(kb) = proc_status_kb(pid, "VmRSS:") {
                peak_rss_kb = peak_rss_kb.max(kb);
            }
            if let Some(kb) = proc_pss_kb(pid) {
                peak_pss_kb = peak_pss_kb.max(kb);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // Read ticks while the process is still alive: /proc/<pid> disappears
        // once it exits.
        let cpu1 = proc_cpu_ticks(pid).unwrap_or(cpu0);
        let elapsed = t0.elapsed().as_secs_f64().max(0.001);
        cpu_pct = (cpu1.saturating_sub(cpu0) as f64) / clock_ticks() as f64 / elapsed * 100.0;
    }

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut hud_ms = Vec::new();
    let mut health_ms = Vec::new();
    let mut progress_ms = Vec::new();
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let Some(rest) = line.strip_prefix("starship_perf ") else {
            continue;
        };
        let fields: BTreeMap<&str, &str> = rest
            .split_whitespace()
            .filter_map(|kv| kv.split_once('='))
            .collect();
        let ms = fields
            .get("ms")
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(f64::NAN);
        match fields.get("kind").copied() {
            Some("hud_toggle") => hud_ms.push(ms),
            Some("event_dispatch") => match fields.get("channel").copied() {
                Some("health") => health_ms.push(ms),
                Some("progress") => progress_ms.push(ms),
                _ => {}
            },
            _ => {}
        }
    }

    let status = child.wait().map_err(|e| format!("wait failed: {e}"))?;
    if !status.success() {
        return Err(format!("probe exited with {status}"));
    }

    Ok(ProbeRun {
        hud_ms,
        health_ms,
        progress_ms,
        peak_rss_mb: peak_rss_kb as f64 / 1024.0,
        peak_pss_mb: peak_pss_kb as f64 / 1024.0,
        cpu_pct,
    })
}

fn p95(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // 95th percentile index without float casts.
    let idx = (values.len() * 95).div_ceil(100);
    values[idx.saturating_sub(1).min(values.len() - 1)]
}

fn main() {
    let args = parse_args();
    println!("perf-budget: binary={}", args.binary.display());

    let mut failures: Vec<String> = Vec::new();

    // Pass 1: idle RSS/CPU.
    let idle = match run_probe(&args, "idle", args.idle_secs) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("perf-budget: idle probe failed: {e}");
            std::process::exit(i32::from(!args.warn_only));
        }
    };
    println!(
        "idle: peak_pss={:.1} MB (budget < {PSS_BUDGET_MB:.0}), peak_rss={:.1} MB (informational; includes shared libraries), cpu={:.2}% (budget < {CPU_BUDGET_PCT:.0})",
        idle.peak_pss_mb, idle.peak_rss_mb, idle.cpu_pct
    );
    if idle.peak_pss_mb > PSS_BUDGET_MB {
        failures.push(format!(
            "idle PSS {:.1} MB > {PSS_BUDGET_MB:.0} MB",
            idle.peak_pss_mb
        ));
    }
    if idle.cpu_pct > CPU_BUDGET_PCT {
        failures.push(format!(
            "idle CPU {:.2}% > {CPU_BUDGET_PCT:.0}%",
            idle.cpu_pct
        ));
    }

    // Pass 2: HUD toggle + event dispatch.
    let latency = match run_probe(&args, "latency", 0) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("perf-budget: latency probe failed: {e}");
            std::process::exit(i32::from(!args.warn_only));
        }
    };
    let mut hud = latency.hud_ms.clone();
    let mut health = latency.health_ms.clone();
    let mut progress = latency.progress_ms.clone();
    let hud_p95 = p95(&mut hud);
    let health_p95 = p95(&mut health);
    let progress_p95 = p95(&mut progress);
    println!(
        "hud_toggle: n={} p95={:.2} ms (budget < {HUD_BUDGET_MS:.0})",
        latency.hud_ms.len(),
        hud_p95
    );
    println!(
        "event_dispatch: health n={} p95={:.3} ms, progress n={} p95={:.3} ms (budget < {EVENT_BUDGET_MS:.0})",
        latency.health_ms.len(),
        health_p95,
        latency.progress_ms.len(),
        progress_p95
    );
    if hud_p95.is_nan() || hud_p95 > HUD_BUDGET_MS {
        failures.push(format!(
            "HUD toggle p95 {hud_p95:.2} ms > {HUD_BUDGET_MS:.0} ms"
        ));
    }
    if health_p95.is_nan() || health_p95 > EVENT_BUDGET_MS {
        failures.push(format!(
            "health dispatch p95 {health_p95:.3} ms > {EVENT_BUDGET_MS:.0} ms"
        ));
    }
    if progress_p95.is_nan() || progress_p95 > EVENT_BUDGET_MS {
        failures.push(format!(
            "progress dispatch p95 {progress_p95:.3} ms > {EVENT_BUDGET_MS:.0} ms"
        ));
    }

    if failures.is_empty() {
        println!("perf-budget: PASS");
    } else {
        for f in &failures {
            eprintln!("perf-budget: FAIL {f}");
        }
        if args.warn_only {
            println!("perf-budget: WARN (warn-only mode, not failing)");
        } else {
            std::process::exit(1);
        }
    }
}
