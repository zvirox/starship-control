use crate::sample::HealthSample;
use serde::{Deserialize, Serialize};

/// System health severity state (GAP-CLOSURE-PLAN.md §14.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum HealthState {
    #[default]
    Normal = 0,
    Elevated = 1,
    Constrained = 2,
    Critical = 3,
}

impl HealthState {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Elevated => "Elevated",
            Self::Constrained => "Constrained",
            Self::Critical => "Critical",
        }
    }
}

/// Alert event emitted when an overload transition or signal trip occurs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthAlert {
    pub signal: String,
    pub level: HealthState,
    pub value: f64,
    pub action: String,
}

/// Configurable thresholds for the overload detector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverloadThresholds {
    pub cpu_elevated_pct: f32,
    pub cpu_constrained_pct: f32,
    pub cpu_critical_pct: f32,
    pub cpu_exit_pct: f32,

    pub psi_elevated: f64,
    pub psi_constrained: f64,
    pub psi_critical: f64,
    pub psi_exit: f64,

    pub mem_avail_elevated_pct: f32,
    pub mem_avail_constrained_pct: f32,
    pub mem_avail_critical_pct: f32,
    pub mem_avail_exit_pct: f32,
    pub swap_in_critical_bytes_per_sec: f64,

    pub disk_free_elevated_pct: f32,
    pub disk_free_constrained_pct: f32,
    pub disk_free_critical_pct: f32,
    pub disk_free_exit_pct: f32,

    pub thermal_delta_elevated: f32,
    pub thermal_delta_constrained: f32,
    pub thermal_delta_critical: f32,
    pub thermal_delta_exit: f32,

    pub vram_elevated_pct: f32,
    pub vram_constrained_pct: f32,
    pub vram_critical_pct: f32,
    pub vram_exit_pct: f32,

    pub enter_consecutive_samples: usize,
    pub exit_consecutive_samples: usize,
}

impl Default for OverloadThresholds {
    fn default() -> Self {
        Self {
            cpu_elevated_pct: 80.0,
            cpu_constrained_pct: 90.0,
            cpu_critical_pct: 97.0,
            cpu_exit_pct: 70.0,

            psi_elevated: 20.0,
            psi_constrained: 40.0,
            psi_critical: 60.0,
            psi_exit: 10.0,

            mem_avail_elevated_pct: 20.0,
            mem_avail_constrained_pct: 10.0,
            mem_avail_critical_pct: 5.0,
            mem_avail_exit_pct: 25.0,
            swap_in_critical_bytes_per_sec: 50.0 * 1024.0 * 1024.0, // 50 MB/s

            disk_free_elevated_pct: 10.0,
            disk_free_constrained_pct: 5.0,
            disk_free_critical_pct: 2.0,
            disk_free_exit_pct: 15.0,

            thermal_delta_elevated: 15.0,
            thermal_delta_constrained: 10.0,
            thermal_delta_critical: 5.0,
            thermal_delta_exit: 20.0,

            vram_elevated_pct: 85.0,
            vram_constrained_pct: 92.0,
            vram_critical_pct: 97.0,
            vram_exit_pct: 80.0,

            enter_consecutive_samples: 3,
            exit_consecutive_samples: 30,
        }
    }
}

/// Signal detector state tracker with hysteresis counter.
#[derive(Debug, Clone, Default)]
struct SignalTracker {
    current_state: HealthState,
    enter_counter: usize,
    exit_counter: usize,
}

/// Overload detector maintaining state across all signals with anti-flapping hysteresis.
#[derive(Debug, Clone, Default)]
pub struct OverloadDetector {
    thresholds: OverloadThresholds,
    overall_state: HealthState,
    cpu_tracker: SignalTracker,
    psi_tracker: SignalTracker,
    mem_tracker: SignalTracker,
    disk_tracker: SignalTracker,
    thermal_tracker: SignalTracker,
    vram_tracker: SignalTracker,
}

impl OverloadDetector {
    #[must_use]
    pub fn new(thresholds: OverloadThresholds) -> Self {
        Self {
            thresholds,
            overall_state: HealthState::Normal,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn current_state(&self) -> HealthState {
        self.overall_state
    }

    #[must_use]
    pub fn thresholds(&self) -> &OverloadThresholds {
        &self.thresholds
    }

    pub fn set_thresholds(&mut self, thresholds: OverloadThresholds) {
        self.thresholds = thresholds;
    }

    /// Process a new `HealthSample`, update internal hysteresis trackers, and return
    /// the new `HealthState` along with any triggered `HealthAlert`s.
    pub fn update(&mut self, sample: &HealthSample) -> (HealthState, Vec<HealthAlert>) {
        let mut alerts = Vec::new();

        // 1. Evaluate CPU signal
        if let Some(cpu) = &sample.cpu {
            let val = f64::from(cpu.global_util_pct);
            let instant_state = if cpu.global_util_pct >= self.thresholds.cpu_critical_pct {
                HealthState::Critical
            } else if cpu.global_util_pct >= self.thresholds.cpu_constrained_pct {
                HealthState::Constrained
            } else if cpu.global_util_pct >= self.thresholds.cpu_elevated_pct {
                HealthState::Elevated
            } else {
                HealthState::Normal
            };
            let is_exit = cpu.global_util_pct < self.thresholds.cpu_exit_pct;

            if let Some(alert) = Self::step_tracker(
                &mut self.cpu_tracker,
                "cpu",
                instant_state,
                is_exit,
                val,
                self.thresholds.enter_consecutive_samples,
                self.thresholds.exit_consecutive_samples,
            ) {
                alerts.push(alert);
            }
        }

        // 2. Evaluate PSI signal
        if let Some(psi) = &sample.psi {
            let val = psi.cpu_some_avg10;
            let instant_state = if val >= self.thresholds.psi_critical {
                HealthState::Critical
            } else if val >= self.thresholds.psi_constrained {
                HealthState::Constrained
            } else if val >= self.thresholds.psi_elevated {
                HealthState::Elevated
            } else {
                HealthState::Normal
            };
            let is_exit = val < self.thresholds.psi_exit;

            if let Some(alert) = Self::step_tracker(
                &mut self.psi_tracker,
                "psi",
                instant_state,
                is_exit,
                val,
                self.thresholds.enter_consecutive_samples,
                self.thresholds.exit_consecutive_samples,
            ) {
                alerts.push(alert);
            }
        }

        // 3. Evaluate Memory signal (MemAvailable % and swap-in rate)
        if let Some(mem) = &sample.memory {
            let val = f64::from(mem.available_pct);
            let instant_state = if mem.available_pct <= self.thresholds.mem_avail_critical_pct
                || mem.swap_in_bytes_per_sec >= self.thresholds.swap_in_critical_bytes_per_sec
            {
                HealthState::Critical
            } else if mem.available_pct <= self.thresholds.mem_avail_constrained_pct {
                HealthState::Constrained
            } else if mem.available_pct <= self.thresholds.mem_avail_elevated_pct {
                HealthState::Elevated
            } else {
                HealthState::Normal
            };
            let is_exit = mem.available_pct > self.thresholds.mem_avail_exit_pct
                && mem.swap_in_bytes_per_sec < self.thresholds.swap_in_critical_bytes_per_sec;

            if let Some(alert) = Self::step_tracker(
                &mut self.mem_tracker,
                "memory",
                instant_state,
                is_exit,
                val,
                self.thresholds.enter_consecutive_samples,
                self.thresholds.exit_consecutive_samples,
            ) {
                alerts.push(alert);
            }
        }

        // 4. Evaluate Disk signal
        if let Some(disk) = &sample.disk {
            let val = f64::from(disk.free_pct);
            let instant_state = if disk.free_pct <= self.thresholds.disk_free_critical_pct {
                HealthState::Critical
            } else if disk.free_pct <= self.thresholds.disk_free_constrained_pct {
                HealthState::Constrained
            } else if disk.free_pct <= self.thresholds.disk_free_elevated_pct {
                HealthState::Elevated
            } else {
                HealthState::Normal
            };
            let is_exit = disk.free_pct > self.thresholds.disk_free_exit_pct;

            if let Some(alert) = Self::step_tracker(
                &mut self.disk_tracker,
                "disk",
                instant_state,
                is_exit,
                val,
                self.thresholds.enter_consecutive_samples,
                self.thresholds.exit_consecutive_samples,
            ) {
                alerts.push(alert);
            }
        }

        // 5. Evaluate Thermal signal (delta to max)
        if let Some(thermal) = &sample.thermal {
            let val = f64::from(thermal.delta_to_max);
            let instant_state = if thermal.delta_to_max <= self.thresholds.thermal_delta_critical {
                HealthState::Critical
            } else if thermal.delta_to_max <= self.thresholds.thermal_delta_constrained {
                HealthState::Constrained
            } else if thermal.delta_to_max <= self.thresholds.thermal_delta_elevated {
                HealthState::Elevated
            } else {
                HealthState::Normal
            };
            let is_exit = thermal.delta_to_max > self.thresholds.thermal_delta_exit;

            if let Some(alert) = Self::step_tracker(
                &mut self.thermal_tracker,
                "thermal",
                instant_state,
                is_exit,
                val,
                self.thresholds.enter_consecutive_samples,
                self.thresholds.exit_consecutive_samples,
            ) {
                alerts.push(alert);
            }
        }

        // 6. Evaluate VRAM signal
        if let Some(gpu) = &sample.gpu {
            let val = f64::from(gpu.vram_used_pct);
            let instant_state = if gpu.vram_used_pct >= self.thresholds.vram_critical_pct {
                HealthState::Critical
            } else if gpu.vram_used_pct >= self.thresholds.vram_constrained_pct {
                HealthState::Constrained
            } else if gpu.vram_used_pct >= self.thresholds.vram_elevated_pct {
                HealthState::Elevated
            } else {
                HealthState::Normal
            };
            let is_exit = gpu.vram_used_pct < self.thresholds.vram_exit_pct;

            if let Some(alert) = Self::step_tracker(
                &mut self.vram_tracker,
                "vram",
                instant_state,
                is_exit,
                val,
                self.thresholds.enter_consecutive_samples,
                self.thresholds.exit_consecutive_samples,
            ) {
                alerts.push(alert);
            }
        }

        // Compute overall state as the max of all active signal states
        let highest_state = *[
            self.cpu_tracker.current_state,
            self.psi_tracker.current_state,
            self.mem_tracker.current_state,
            self.disk_tracker.current_state,
            self.thermal_tracker.current_state,
            self.vram_tracker.current_state,
        ]
        .iter()
        .max()
        .unwrap_or(&HealthState::Normal);

        self.overall_state = highest_state;
        (self.overall_state, alerts)
    }

    /// Generic step function with hysteresis for a single signal tracker.
    fn step_tracker(
        tracker: &mut SignalTracker,
        signal_name: &str,
        instant_state: HealthState,
        is_exit: bool,
        value: f64,
        enter_threshold: usize,
        exit_threshold: usize,
    ) -> Option<HealthAlert> {
        let old_state = tracker.current_state;

        if instant_state > tracker.current_state {
            // Escalation path: requires `enter_threshold` consecutive samples
            tracker.enter_counter += 1;
            tracker.exit_counter = 0;

            if tracker.enter_counter >= enter_threshold {
                tracker.current_state = instant_state;
                tracker.enter_counter = 0;

                let action = match instant_state {
                    HealthState::Elevated => "warn",
                    HealthState::Constrained => "throttle",
                    HealthState::Critical => "safe_pause",
                    HealthState::Normal => "none",
                };

                return Some(HealthAlert {
                    signal: signal_name.to_string(),
                    level: instant_state,
                    value,
                    action: action.to_string(),
                });
            }
        } else if is_exit && tracker.current_state > HealthState::Normal {
            // De-escalation path: requires `exit_threshold` consecutive quiet samples
            tracker.exit_counter += 1;
            tracker.enter_counter = 0;

            if tracker.exit_counter >= exit_threshold {
                // Step down one level
                let new_state = match tracker.current_state {
                    HealthState::Critical => HealthState::Constrained,
                    HealthState::Constrained => HealthState::Elevated,
                    _ => HealthState::Normal,
                };
                tracker.current_state = new_state;
                tracker.exit_counter = 0;

                if new_state != old_state {
                    return Some(HealthAlert {
                        signal: signal_name.to_string(),
                        level: new_state,
                        value,
                        action: "deescalate".to_string(),
                    });
                }
            }
        } else {
            // In between enter and exit thresholds: keep current state, reset counters
            tracker.enter_counter = 0;
            tracker.exit_counter = 0;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::CpuMetrics;

    #[test]
    fn test_hysteresis_anti_flapping_sustaining_elevated() {
        // Verification requirement for 14.6:
        // "Verify: test matrix with artificial sample streams (flapping CPU around 80 %
        // does NOT toggle state repeatedly; sustains Elevated until exit threshold met)."
        let thresholds = OverloadThresholds {
            enter_consecutive_samples: 3,
            exit_consecutive_samples: 10,
            cpu_elevated_pct: 80.0,
            cpu_exit_pct: 70.0,
            ..Default::default()
        };

        let mut detector = OverloadDetector::new(thresholds);
        assert_eq!(detector.current_state(), HealthState::Normal);

        // Helper to produce a CPU sample with a given util
        let make_cpu_sample = |util: f32| HealthSample {
            cpu: Some(CpuMetrics {
                global_util_pct: util,
                core_util_pct: vec![util],
                load_avg_1m: 1.0,
                load_avg_5m: 1.0,
                load_avg_15m: 1.0,
                avg_freq_mhz: 3000,
            }),
            ..Default::default()
        };

        // 1. Two samples above 80% - should NOT enter Elevated yet (requires 3)
        let (s1, _) = detector.update(&make_cpu_sample(82.0));
        assert_eq!(s1, HealthState::Normal);
        let (s2, _) = detector.update(&make_cpu_sample(81.0));
        assert_eq!(s2, HealthState::Normal);

        // 2. Third sample above 80% - ENTERS Elevated
        let (s3, alerts) = detector.update(&make_cpu_sample(85.0));
        assert_eq!(s3, HealthState::Elevated);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].action, "warn");

        // 3. FLAPPING TEST: oscillate around 80% (79%, 81%, 78%, 82%, 75%)
        // Because none of these are below the exit threshold of 70%, it MUST sustain Elevated!
        let flapping_values = [79.0, 81.0, 78.0, 82.0, 75.0, 79.5, 76.0, 80.5];
        for val in flapping_values {
            let (state, a) = detector.update(&make_cpu_sample(val));
            assert_eq!(
                state,
                HealthState::Elevated,
                "Flapping at {val}% must not change state from Elevated"
            );
            assert!(
                a.is_empty(),
                "No alert should be raised while sustaining Elevated"
            );
        }

        // 4. Drops below exit threshold (65.0 < 70.0), but only for 5 samples (requires 10)
        for _ in 0..5 {
            let (state, _) = detector.update(&make_cpu_sample(65.0));
            assert_eq!(
                state,
                HealthState::Elevated,
                "Still requires 10 exit samples"
            );
        }

        // Brief spike back to 75% resets the exit counter
        let (state, _) = detector.update(&make_cpu_sample(75.0));
        assert_eq!(state, HealthState::Elevated);

        // 5. Sustained 10 consecutive quiet samples below 70% (< 70%)
        for i in 1..=9 {
            let (state, _) = detector.update(&make_cpu_sample(60.0));
            assert_eq!(
                state,
                HealthState::Elevated,
                "Waiting for 10th sample: step {i}"
            );
        }

        // On the 10th consecutive quiet sample, it drops to Normal!
        let (state, deescalate_alerts) = detector.update(&make_cpu_sample(60.0));
        assert_eq!(state, HealthState::Normal);
        assert_eq!(deescalate_alerts.len(), 1);
        assert_eq!(deescalate_alerts[0].action, "deescalate");
    }
}
