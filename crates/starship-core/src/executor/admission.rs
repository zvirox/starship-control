use serde::{Deserialize, Serialize};
use starship_health::HealthSample;
use starship_plugin::{CpuLevel, ResourceProfile};
use std::collections::HashMap;

/// Admission evaluation decision for a candidate step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionDecision {
    /// Resources are sufficient; step is admitted to run.
    Admit,
    /// Resources are insufficient; step is queued.
    Queued { reason: String },
}

/// Active resource reservation held by a running step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveReservation {
    pub step_id: i64,
    pub tool_name: String,
    pub profile: ResourceProfile,
    pub granted_at_ms: u64,
}

/// Admission Controller that guards desktop responsiveness and prevents thrashing
/// by admitting steps only when live headroom is sufficient (GAP-CLOSURE-PLAN.md §15.2).
#[derive(Debug, Clone)]
pub struct AdmissionController {
    reservations: HashMap<i64, ActiveReservation>,
    max_cpu_high_threshold_pct: f32,
    max_cpu_med_threshold_pct: f32,
    max_cpu_low_threshold_pct: f32,
    min_headroom_high_pct: f32,
    min_headroom_med_pct: f32,
    min_headroom_low_pct: f32,
}

impl Default for AdmissionController {
    fn default() -> Self {
        Self {
            reservations: HashMap::new(),
            max_cpu_high_threshold_pct: 80.0,
            max_cpu_med_threshold_pct: 90.0,
            max_cpu_low_threshold_pct: 98.0,
            min_headroom_high_pct: 25.0,
            min_headroom_med_pct: 10.0,
            min_headroom_low_pct: 2.0,
        }
    }
}

impl AdmissionController {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of currently active reservations.
    #[must_use]
    pub fn active_reservations_count(&self) -> usize {
        self.reservations.len()
    }

    /// Evaluates whether the system has sufficient headroom to admit a tool with the given profile.
    #[must_use]
    pub fn evaluate(
        &self,
        profile: &ResourceProfile,
        health: Option<&HealthSample>,
    ) -> AdmissionDecision {
        // 1. Extract live health metrics
        let live_cpu = health
            .and_then(|h| h.cpu.as_ref())
            .map_or(0.0, |c| c.global_util_pct);

        let live_mem_avail_mb = health
            .and_then(|h| h.memory.as_ref())
            .map_or(16_384, |m| m.available_bytes / (1024 * 1024));

        let live_gpu_busy = health
            .and_then(|h| h.gpu.as_ref())
            .is_some_and(|g| g.util_pct >= 90.0);

        // 2. Sum existing reservations
        let mut reserved_cpu_pct = 0.0;
        let mut reserved_mem_mb = 0;
        let mut reserved_gpu = false;

        for res in self.reservations.values() {
            match res.profile.cpu {
                CpuLevel::High => reserved_cpu_pct += 50.0,
                CpuLevel::Med => reserved_cpu_pct += 25.0,
                CpuLevel::Low => reserved_cpu_pct += 5.0,
            }
            reserved_mem_mb += res.profile.mem_mb;
            if res.profile.gpu {
                reserved_gpu = true;
            }
        }

        let total_est_cpu = live_cpu + reserved_cpu_pct;
        let headroom_cpu = (100.0 - total_est_cpu).max(0.0);
        let headroom_mem_mb = live_mem_avail_mb.saturating_sub(reserved_mem_mb);

        // 3. Evaluate CPU threshold & headroom
        match profile.cpu {
            CpuLevel::High => {
                if live_cpu >= self.max_cpu_high_threshold_pct {
                    return AdmissionDecision::Queued {
                        reason: format!(
                            "waiting for resources: system CPU utilization too high ({live_cpu:.1}%), required High profile headroom"
                        ),
                    };
                }
                if headroom_cpu < self.min_headroom_high_pct {
                    return AdmissionDecision::Queued {
                        reason: format!(
                            "waiting for resources: insufficient CPU headroom ({headroom_cpu:.1}% remaining), required High profile headroom"
                        ),
                    };
                }
            }
            CpuLevel::Med => {
                if live_cpu >= self.max_cpu_med_threshold_pct {
                    return AdmissionDecision::Queued {
                        reason: format!(
                            "waiting for resources: system CPU utilization too high ({live_cpu:.1}%), required Med profile headroom"
                        ),
                    };
                }
                if headroom_cpu < self.min_headroom_med_pct {
                    return AdmissionDecision::Queued {
                        reason: format!(
                            "waiting for resources: insufficient CPU headroom ({headroom_cpu:.1}% remaining), required Med profile headroom"
                        ),
                    };
                }
            }
            CpuLevel::Low => {
                if live_cpu >= self.max_cpu_low_threshold_pct {
                    return AdmissionDecision::Queued {
                        reason: format!(
                            "waiting for resources: system CPU critically saturated ({live_cpu:.1}%)"
                        ),
                    };
                }
                if headroom_cpu < self.min_headroom_low_pct {
                    return AdmissionDecision::Queued {
                        reason: format!(
                            "waiting for resources: minimal CPU headroom unavailable ({headroom_cpu:.1}% remaining)"
                        ),
                    };
                }
            }
        }

        // 4. Evaluate RAM headroom
        if headroom_mem_mb < profile.mem_mb {
            return AdmissionDecision::Queued {
                reason: format!(
                    "waiting for resources: insufficient RAM headroom ({headroom_mem_mb} MB available < {} MB required)",
                    profile.mem_mb
                ),
            };
        }

        // 5. Evaluate GPU exclusivity
        if profile.gpu && (reserved_gpu || live_gpu_busy) {
            return AdmissionDecision::Queued {
                reason: "waiting for resources: GPU accelerator currently occupied".to_string(),
            };
        }

        AdmissionDecision::Admit
    }

    /// Evaluates headroom and, if admitted, records the reservation for the step.
    pub fn try_admit(
        &mut self,
        step_id: i64,
        tool_name: impl Into<String>,
        profile: ResourceProfile,
        health: Option<&HealthSample>,
        now_ms: u64,
    ) -> AdmissionDecision {
        let decision = self.evaluate(&profile, health);
        if decision == AdmissionDecision::Admit {
            self.reservations.insert(
                step_id,
                ActiveReservation {
                    step_id,
                    tool_name: tool_name.into(),
                    profile,
                    granted_at_ms: now_ms,
                },
            );
        }
        decision
    }

    /// Releases a step's reservation once execution completes, fails, or is cancelled.
    pub fn release(&mut self, step_id: i64) -> Option<ActiveReservation> {
        self.reservations.remove(&step_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starship_health::{CpuMetrics, MemoryMetrics};
    use starship_plugin::DiskIoLevel;

    fn make_health(cpu_pct: f32, avail_mem_mb: u64) -> HealthSample {
        let mut sample = HealthSample::default();
        sample.cpu = Some(CpuMetrics {
            global_util_pct: cpu_pct,
            core_util_pct: vec![cpu_pct],
            load_avg_1m: f64::from(cpu_pct) / 10.0,
            load_avg_5m: 1.0,
            load_avg_15m: 1.0,
            avg_freq_mhz: 3200,
        });
        sample.memory = Some(MemoryMetrics {
            total_bytes: 32 * 1024 * 1024 * 1024,
            used_bytes: (32 * 1024 - avail_mem_mb) * 1024 * 1024,
            available_bytes: avail_mem_mb * 1024 * 1024,
            available_pct: (avail_mem_mb as f32 / (32.0 * 1024.0)) * 100.0,
            swap_total_bytes: 8 * 1024 * 1024 * 1024,
            swap_used_bytes: 0,
            swap_in_bytes_per_sec: 0.0,
        });
        sample
    }

    #[test]
    fn test_admission_at_85_pct_cpu_queues_high_and_runs_low() {
        let mut controller = AdmissionController::new();
        let health_85 = make_health(85.0, 16_384);

        let high_step_profile = ResourceProfile {
            cpu: CpuLevel::High,
            mem_mb: 2048,
            gpu: false,
            disk_io: DiskIoLevel::High,
            long_running: true,
        };

        let low_step_profile = ResourceProfile {
            cpu: CpuLevel::Low,
            mem_mb: 256,
            gpu: false,
            disk_io: DiskIoLevel::Low,
            long_running: false,
        };

        // 1. High step attempts admission at 85% CPU -> Queued
        let decision_high = controller.try_admit(
            1,
            "eep.timeline.export_mp4",
            high_step_profile,
            Some(&health_85),
            1_000,
        );
        match decision_high {
            AdmissionDecision::Queued { reason } => {
                assert!(
                    reason.contains("waiting for resources"),
                    "Expected waiting for resources, got {reason}"
                );
                assert!(reason.contains("85.0%"));
            }
            AdmissionDecision::Admit => panic!("High step should not be admitted at 85% CPU"),
        }
        assert_eq!(controller.active_reservations_count(), 0);

        // 2. Low step attempts admission at 85% CPU -> Admitted
        let decision_low = controller.try_admit(
            2,
            "ssp.recording.status",
            low_step_profile,
            Some(&health_85),
            1_050,
        );
        assert_eq!(decision_low, AdmissionDecision::Admit);
        assert_eq!(controller.active_reservations_count(), 1);

        // Release reservation
        let released = controller.release(2);
        assert!(released.is_some());
        assert_eq!(controller.active_reservations_count(), 0);
    }

    #[test]
    fn test_admission_reservations_drain_headroom() {
        let mut controller = AdmissionController::new();
        // Machine has 40% CPU idle (live_cpu = 40.0)
        let health_40 = make_health(40.0, 8192);

        let high_profile = ResourceProfile {
            cpu: CpuLevel::High,
            mem_mb: 1024,
            gpu: false,
            disk_io: DiskIoLevel::High,
            long_running: true,
        };

        // First high step admitted (reserves 50% CPU, total estimated = 90%)
        let d1 = controller.try_admit(10, "tool.heavy.1", high_profile.clone(), Some(&health_40), 100);
        assert_eq!(d1, AdmissionDecision::Admit);

        // Second high step queued due to lack of CPU headroom
        let d2 = controller.try_admit(11, "tool.heavy.2", high_profile, Some(&health_40), 120);
        assert!(matches!(d2, AdmissionDecision::Queued { .. }));

        // Release first step
        controller.release(10);

        // Now second high step can be admitted
        let d3 = controller.try_admit(11, "tool.heavy.2", ResourceProfile::default(), Some(&health_40), 150);
        assert_eq!(d3, AdmissionDecision::Admit);
    }
}
