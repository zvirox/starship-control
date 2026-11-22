use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use starship_core::executor::{Governor, GovernorDirective};
use starship_db::{Database, HealthRollupRecord};
use starship_health::{
    HealthAlert, HealthMonitor, HealthRingBuffer, HealthSample, HealthState, OverloadDetector,
    OverloadThresholds, aggregate_samples,
};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tracing::{debug, info};

/// Response payload for `cmd://health_get_snapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSnapshotResponse {
    pub sample: HealthSample,
    pub state: HealthState,
    pub governor_directive: GovernorDirective,
}

/// Request payload for `cmd://health_get_history`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthHistoryPayload {
    pub range: String,
    pub resolution: Option<String>,
}

/// Response payload for `cmd://health_get_history`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthHistoryResponse {
    pub range: String,
    pub samples: Vec<HealthSample>,
    pub rollups: Vec<HealthRollupRecord>,
}

/// Event payload emitted on `evt://health/state_changed`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthStateChangedEvent {
    pub previous_state: HealthState,
    pub current_state: HealthState,
    pub directive: GovernorDirective,
}

/// Unified Health Service coordinating monitoring, detection, governor actions,
/// in-memory sparkline buffer, and SQLite persistence (GAP-CLOSURE-PLAN.md §14.8).
pub struct HealthService {
    monitor: Mutex<HealthMonitor>,
    detector: Mutex<OverloadDetector>,
    governor: Mutex<Governor>,
    ring_buffer: Mutex<HealthRingBuffer>,
    last_state: Mutex<HealthState>,
    last_minute_rollup_ts: Mutex<i64>,
}

impl Default for HealthService {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthService {
    #[must_use]
    pub fn new() -> Self {
        let thresholds = OverloadThresholds::default();
        Self {
            monitor: Mutex::new(HealthMonitor::new()),
            detector: Mutex::new(OverloadDetector::new(thresholds)),
            governor: Mutex::new(Governor::default()),
            ring_buffer: Mutex::new(HealthRingBuffer::default()),
            last_state: Mutex::new(HealthState::Normal),
            last_minute_rollup_ts: Mutex::new(0),
        }
    }

    /// Obtains an instantaneous snapshot of system metrics, health state, and governor directive.
    pub fn get_snapshot(&self) -> Result<HealthSnapshotResponse, CoreError> {
        let mut monitor = self.monitor.lock().map_err(|_| {
            CoreError::ipc(
                "health.lock_poisoned",
                false,
                "Health monitor lock poisoned",
                None,
            )
        })?;
        let sample = monitor.sample();

        let mut detector = self.detector.lock().map_err(|_| {
            CoreError::ipc(
                "health.lock_poisoned",
                false,
                "Health detector lock poisoned",
                None,
            )
        })?;
        let (state, _) = detector.update(&sample);

        let mut governor = self.governor.lock().map_err(|_| {
            CoreError::ipc(
                "health.lock_poisoned",
                false,
                "Health governor lock poisoned",
                None,
            )
        })?;
        let governor_directive = governor.on_health_update(state);

        // Record into in-memory ring buffer
        if let Ok(mut ring) = self.ring_buffer.lock() {
            ring.push(sample.clone());
        }

        Ok(HealthSnapshotResponse {
            sample,
            state,
            governor_directive,
        })
    }

    /// Fetches historical metrics from the in-memory ring buffer (for 15m) or SQLite rollups.
    pub fn get_history(
        &self,
        payload: HealthHistoryPayload,
        db: &Database,
    ) -> Result<HealthHistoryResponse, CoreError> {
        let range = payload.range.as_str();
        let now_s = current_time_s();

        match range {
            "15m" => {
                let ring = self.ring_buffer.lock().map_err(|_| {
                    CoreError::ipc(
                        "health.lock_poisoned",
                        false,
                        "Ring buffer lock poisoned",
                        None,
                    )
                })?;
                let samples = ring.last_n_minutes(15);
                Ok(HealthHistoryResponse {
                    range: payload.range,
                    samples,
                    rollups: Vec::new(),
                })
            }
            "1h" => {
                let from_ts = now_s - 3600;
                let rollups = db.get_health_rollups_1m(from_ts, now_s)?;
                let ring = self.ring_buffer.lock().map_err(|_| {
                    CoreError::ipc(
                        "health.lock_poisoned",
                        false,
                        "Ring buffer lock poisoned",
                        None,
                    )
                })?;
                let samples = ring.last_n_minutes(15);
                Ok(HealthHistoryResponse {
                    range: payload.range,
                    samples,
                    rollups,
                })
            }
            "24h" => {
                let from_ts = now_s - 86400;
                let rollups = db.get_health_rollups_1m(from_ts, now_s)?;
                Ok(HealthHistoryResponse {
                    range: payload.range,
                    samples: Vec::new(),
                    rollups,
                })
            }
            "7d" => {
                let from_ts = now_s - (7 * 86400);
                let rollups = db.get_health_rollups_1h(from_ts, now_s)?;
                Ok(HealthHistoryResponse {
                    range: payload.range,
                    samples: Vec::new(),
                    rollups,
                })
            }
            "30d" | "90d" => {
                let days = if range == "30d" { 30 } else { 90 };
                let from_ts = now_s - (days * 86400);
                let rollups = db.get_health_rollups_1h(from_ts, now_s)?;
                Ok(HealthHistoryResponse {
                    range: payload.range,
                    samples: Vec::new(),
                    rollups,
                })
            }
            _ => {
                let ring = self.ring_buffer.lock().map_err(|_| {
                    CoreError::ipc(
                        "health.lock_poisoned",
                        false,
                        "Ring buffer lock poisoned",
                        None,
                    )
                })?;
                let samples = ring.last_n_minutes(15);
                Ok(HealthHistoryResponse {
                    range: "15m".to_string(),
                    samples,
                    rollups: Vec::new(),
                })
            }
        }
    }

    /// Obtains currently configured overload thresholds.
    pub fn get_thresholds(&self) -> Result<OverloadThresholds, CoreError> {
        let detector = self.detector.lock().map_err(|_| {
            CoreError::ipc(
                "health.lock_poisoned",
                false,
                "Health detector lock poisoned",
                None,
            )
        })?;
        Ok(detector.thresholds().clone())
    }

    /// Updates overload thresholds.
    pub fn set_thresholds(
        &self,
        thresholds: OverloadThresholds,
    ) -> Result<OverloadThresholds, CoreError> {
        let mut detector = self.detector.lock().map_err(|_| {
            CoreError::ipc(
                "health.lock_poisoned",
                false,
                "Health detector lock poisoned",
                None,
            )
        })?;
        detector.set_thresholds(thresholds.clone());
        info!(target: "starship_health", "Overload detector thresholds updated");
        Ok(thresholds)
    }

    /// Periodic sampling tick (runs at 1 Hz when active or 0.2 Hz when idle).
    /// Emits events and records 1-minute rollups into SQLite.
    pub fn tick(&self, app: &AppHandle, db: &Database) {
        let Ok(mut monitor) = self.monitor.lock() else {
            return;
        };
        let sample = monitor.sample();
        drop(monitor);

        // Store into in-memory ring buffer
        if let Ok(mut ring) = self.ring_buffer.lock() {
            ring.push(sample.clone());
        }

        // Emit 1 Hz sample event
        let _ = app.emit("evt://health/sample", &sample);

        // Update overload detector
        let Ok(mut detector) = self.detector.lock() else {
            return;
        };
        let (current_state, alerts) = detector.update(&sample);
        drop(detector);

        // Forward alerts
        for alert in &alerts {
            let _ = app.emit("evt://health/alert", alert);
            let now_s = current_time_s();
            let _ = db.record_health_alert(
                now_s,
                &alert.signal,
                alert.level.as_str(),
                alert.value,
                &alert.action,
                None,
            );
        }

        // Update governor and check for state transition
        let Ok(mut governor) = self.governor.lock() else {
            return;
        };
        let directive = governor.on_health_update(current_state);
        drop(governor);

        let Ok(mut last_state_guard) = self.last_state.lock() else {
            return;
        };
        if *last_state_guard != current_state {
            let prev = *last_state_guard;
            *last_state_guard = current_state;
            debug!(
                target: "starship_health",
                previous = ?prev,
                current = ?current_state,
                "Health state changed"
            );
            let _ = app.emit(
                "evt://health/state_changed",
                HealthStateChangedEvent {
                    previous_state: prev,
                    current_state,
                    directive,
                },
            );

            // Tray state synchronization for Overloaded state (GAP-CLOSURE-PLAN.md §14.10)
            if current_state == HealthState::Critical || current_state == HealthState::Constrained {
                let _ = app.emit(
                    "evt://tray/state_changed",
                    crate::ipc::commands::TraySetStatePayload {
                        state: crate::ipc::commands::TrayState::Overloaded,
                    },
                );
                if let Some(tray) = app.tray_by_id("main-tray") {
                    let _ = tray.set_tooltip(Some(format!(
                        "Starship Control — {}",
                        crate::ipc::commands::TrayState::Overloaded.label()
                    )));
                }
            } else if prev == HealthState::Critical || prev == HealthState::Constrained {
                let _ = app.emit(
                    "evt://tray/state_changed",
                    crate::ipc::commands::TraySetStatePayload {
                        state: crate::ipc::commands::TrayState::Idle,
                    },
                );
                if let Some(tray) = app.tray_by_id("main-tray") {
                    let _ = tray.set_tooltip(Some(format!(
                        "Starship Control — {}",
                        crate::ipc::commands::TrayState::Idle.label()
                    )));
                }
            }
        }

        // Periodic 1-minute rollup generation
        let now_s = current_time_s();
        let Ok(mut last_rollup) = self.last_minute_rollup_ts.lock() else {
            return;
        };
        if now_s - *last_rollup >= 60 {
            *last_rollup = now_s;
            if let Ok(ring) = self.ring_buffer.lock() {
                let recent_samples = ring.last_n_minutes(1);
                if let Some(agg) = aggregate_samples(&recent_samples) {
                    let rollup = HealthRollupRecord {
                        ts: now_s,
                        cpu_avg: agg.cpu_avg,
                        cpu_max: agg.cpu_max,
                        mem_avail_pct_min: agg.mem_avail_pct_min,
                        swap_in_max: agg.swap_in_max,
                        disk_free_pct_min: agg.disk_free_pct_min,
                        temp_max: agg.temp_max,
                        gpu_util_max: agg.gpu_util_max,
                        vram_pct_max: agg.vram_pct_max,
                        psi_cpu_max: agg.psi_cpu_max,
                        psi_mem_max: agg.psi_mem_max,
                        psi_io_max: agg.psi_io_max,
                        state: current_state.as_str().to_string(),
                    };
                    let _ = db.insert_health_rollup_1m(&rollup);
                }
            }
        }
    }

    /// Pure state evaluation without Tauri `AppHandle` dependency, for deterministic unit testing.
    pub fn evaluate_sample_transition(
        &self,
        sample: &HealthSample,
    ) -> (HealthState, Vec<HealthAlert>, GovernorDirective, bool) {
        if let Ok(mut ring) = self.ring_buffer.lock() {
            ring.push(sample.clone());
        }

        let (current_state, alerts) = if let Ok(mut detector) = self.detector.lock() {
            detector.update(sample)
        } else {
            (HealthState::Normal, Vec::new())
        };

        let directive = if let Ok(mut governor) = self.governor.lock() {
            governor.on_health_update(current_state)
        } else {
            GovernorDirective::default()
        };

        let mut state_changed = false;
        if let Ok(mut last_state_guard) = self.last_state.lock()
            && *last_state_guard != current_state
        {
            *last_state_guard = current_state;
            state_changed = true;
        }

        (current_state, alerts, directive, state_changed)
    }
}

fn current_time_s() -> i64 {
    #[allow(clippy::cast_possible_wrap)]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starship_health::{CpuMetrics, MemoryMetrics, OverloadThresholds};
    use std::time::Instant;

    #[test]
    fn test_injected_critical_state_escalates_and_demands_governor_pause_within_two_seconds() {
        // Verification for GAP-CLOSURE-PLAN.md §14.10:
        // "Verify: injected Critical state shows banner within 2 s."
        let service = HealthService::new();

        // Configure detector thresholds with enter_consecutive_samples = 2
        let thresholds = OverloadThresholds {
            enter_consecutive_samples: 2,
            cpu_critical_pct: 95.0,
            ..Default::default()
        };
        let _ = service.set_thresholds(thresholds);

        let critical_sample = HealthSample {
            timestamp_ms: 1_700_000_000_000,
            cpu: Some(CpuMetrics {
                global_util_pct: 98.0,
                core_util_pct: vec![98.0; 4],
                load_avg_1m: 4.5,
                load_avg_5m: 3.2,
                load_avg_15m: 2.1,
                avg_freq_mhz: 3600,
            }),
            memory: Some(MemoryMetrics {
                total_bytes: 16 * 1024 * 1024 * 1024,
                available_bytes: 10 * 1024 * 1024 * 1024,
                used_bytes: 6 * 1024 * 1024 * 1024,
                available_pct: 62.5,
                swap_total_bytes: 8 * 1024 * 1024 * 1024,
                swap_used_bytes: 0,
                swap_in_bytes_per_sec: 0.0,
            }),
            psi: None,
            disk: None,
            network: None,
            thermal: None,
            gpu: None,
            power: None,
            top_processes: Vec::new(),
            own_footprint: None,
        };

        let start = Instant::now();

        // First sample -> enters critical accumulator
        let (s1, _, d1, c1) = service.evaluate_sample_transition(&critical_sample);
        assert_eq!(s1, HealthState::Normal);
        assert!(!d1.should_pause);
        assert!(!c1);

        // Second sample (simulating 1 second later @ 1 Hz) -> trips hysteresis enter threshold!
        let (s2, _, d2, c2) = service.evaluate_sample_transition(&critical_sample);
        let elapsed = start.elapsed();

        assert_eq!(s2, HealthState::Critical);
        assert!(
            d2.should_pause,
            "Governor must demand safe pause on Critical state"
        );
        assert!(
            c2,
            "State change must be flagged to emit evt://health/state_changed and Overloaded tray state"
        );
        assert!(
            elapsed.as_secs_f64() < 2.0,
            "State escalation occurred within 2.0s (measured: {elapsed:?})"
        );
    }
}
