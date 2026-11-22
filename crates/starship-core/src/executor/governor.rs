use crate::executor::state::{MissionEvent, MissionState, PauseReason, transition_mission};
use serde::{Deserialize, Serialize};
use starship_health::HealthState;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Directives emitted by the system governor to control concurrency and workloads (GAP-CLOSURE-PLAN.md §14.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct GovernorDirective {
    pub current_health_state: HealthState,
    pub max_parallelism: usize,
    pub allow_heavy_steps: bool,
    pub allow_local_inference: bool,
    pub allow_background_prefetch: bool,
    pub should_pause: bool,
    pub should_resume: bool,
    pub should_unload_idle_model: bool,
    pub action_message: String,
}

/// System execution governor balancing system health against mission execution.
pub struct Governor {
    current_state: HealthState,
    default_parallelism: usize,
    paused_missions: HashSet<i64>,
    quiet_since: Option<Instant>,
    auto_resume_delay: Duration,
}

impl Default for Governor {
    fn default() -> Self {
        Self::new(2, Duration::from_secs(60))
    }
}

impl Governor {
    #[must_use]
    pub fn new(default_parallelism: usize, auto_resume_delay: Duration) -> Self {
        Self {
            current_state: HealthState::Normal,
            default_parallelism: default_parallelism.max(1),
            paused_missions: HashSet::new(),
            quiet_since: Some(Instant::now()),
            auto_resume_delay,
        }
    }

    #[must_use]
    pub fn current_state(&self) -> HealthState {
        self.current_state
    }

    #[must_use]
    pub fn is_governor_paused(&self, mission_id: i64) -> bool {
        self.paused_missions.contains(&mission_id)
    }

    /// Evaluates incoming system health state and derives the updated governor directive.
    pub fn on_health_update(&mut self, new_state: HealthState) -> GovernorDirective {
        let previous_state = self.current_state;
        self.current_state = new_state;

        // Track quiet period (when state is <= Elevated) for auto-resume timer
        if new_state <= HealthState::Elevated {
            if self.quiet_since.is_none() {
                self.quiet_since = Some(Instant::now());
            }
        } else {
            // Constrained or Critical resets the quiet counter
            self.quiet_since = None;
        }

        let mut should_pause = false;
        let mut should_resume = false;
        let mut max_parallelism = self.default_parallelism;
        let mut allow_heavy_steps = true;
        let mut allow_local_inference = true;
        let mut allow_background_prefetch = true;
        let mut should_unload_idle_model = false;
        let action_message;

        match new_state {
            HealthState::Normal => {
                action_message = "Normal system health: standard execution".to_string();
                if !self.paused_missions.is_empty() && self.can_auto_resume() {
                    should_resume = true;
                }
            }
            HealthState::Elevated => {
                allow_background_prefetch = false;
                action_message = "Elevated system load: background prefetch suspended".to_string();
                if !self.paused_missions.is_empty() && self.can_auto_resume() {
                    should_resume = true;
                }
            }
            HealthState::Constrained => {
                max_parallelism = 1;
                allow_heavy_steps = false;
                allow_background_prefetch = false;
                should_unload_idle_model = true;
                action_message =
                    "Constrained load: parallelism capped to 1, heavy steps blocked".to_string();
            }
            HealthState::Critical => {
                max_parallelism = 0;
                allow_heavy_steps = false;
                allow_local_inference = false;
                allow_background_prefetch = false;
                should_pause = true;
                action_message =
                    "Critical system load: missions safe-paused at step boundary".to_string();
            }
        }

        let _ = previous_state;

        GovernorDirective {
            current_health_state: new_state,
            max_parallelism,
            allow_heavy_steps,
            allow_local_inference,
            allow_background_prefetch,
            should_pause,
            should_resume,
            should_unload_idle_model,
            action_message,
        }
    }

    /// Check if the quiet recovery timer has been met (state <= Elevated for >= `auto_resume_delay`).
    #[must_use]
    pub fn can_auto_resume(&self) -> bool {
        if self.current_state > HealthState::Elevated {
            return false;
        }

        match self.quiet_since {
            Some(start) => start.elapsed() >= self.auto_resume_delay,
            None => false,
        }
    }

    /// Registers a mission as paused by the governor.
    pub fn register_paused_mission(
        &mut self,
        mission_id: i64,
        mission_state: MissionState,
    ) -> Result<MissionState, starship_health::HealthState> {
        self.paused_missions.insert(mission_id);
        transition_mission(mission_state, MissionEvent::Pause(PauseReason::Governor))
            .map_err(|_| self.current_state)
    }

    /// Attempts to auto-resume a mission.
    ///
    /// CRITICAL CONTRACT LAW (GAP-CLOSURE-PLAN.md §14.7):
    /// Auto-resume is permitted ONLY if:
    /// 1. The mission was paused by the Governor (`Paused(PauseReason::Governor)`).
    /// 2. It is tracked in `self.paused_missions`.
    /// 3. Current state has been <= Elevated for >= `auto_resume_delay`.
    ///
    /// Under NO circumstances will user-paused (`Paused(PauseReason::User)`) or
    /// approval-paused (`Paused(PauseReason::Approval)`) missions be auto-resumed.
    #[must_use]
    pub fn try_auto_resume_mission(
        &mut self,
        mission_id: i64,
        mission_state: MissionState,
    ) -> Option<MissionState> {
        if !self.can_auto_resume() {
            return None;
        }

        // Must be in paused_missions set
        if !self.paused_missions.contains(&mission_id) {
            return None;
        }

        // Must be strictly Paused(PauseReason::Governor)
        if mission_state != MissionState::Paused(PauseReason::Governor) {
            return None;
        }

        self.paused_missions.remove(&mission_id);
        transition_mission(mission_state, MissionEvent::Resume).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governor_directives_across_all_health_states() {
        let mut gov = Governor::new(2, Duration::from_millis(50));

        // 1. Normal state
        let d_norm = gov.on_health_update(HealthState::Normal);
        assert_eq!(d_norm.max_parallelism, 2);
        assert!(d_norm.allow_heavy_steps);
        assert!(d_norm.allow_local_inference);
        assert!(d_norm.allow_background_prefetch);
        assert!(!d_norm.should_pause);

        // 2. Elevated state
        let d_elev = gov.on_health_update(HealthState::Elevated);
        assert_eq!(d_elev.max_parallelism, 2);
        assert!(d_elev.allow_heavy_steps);
        assert!(!d_elev.allow_background_prefetch, "Drops prefetch");
        assert!(!d_elev.should_pause);

        // 3. Constrained state
        let d_const = gov.on_health_update(HealthState::Constrained);
        assert_eq!(d_const.max_parallelism, 1, "Parallelism capped to 1");
        assert!(!d_const.allow_heavy_steps, "Heavy steps blocked");
        assert!(d_const.should_unload_idle_model);
        assert!(!d_const.should_pause);

        // 4. Critical state
        let d_crit = gov.on_health_update(HealthState::Critical);
        assert_eq!(d_crit.max_parallelism, 0);
        assert!(!d_crit.allow_local_inference, "Refuses local inference");
        assert!(d_crit.should_pause, "Safe-pause at boundary triggered");
    }

    #[test]
    fn test_governor_auto_resume_only_for_governor_paused_missions_after_cooldown() {
        let mut gov = Governor::new(2, Duration::from_millis(30));

        // Mission 42 running -> Governor Critical safe-pauses it
        let mut m42_state = MissionState::Running;
        m42_state = gov
            .register_paused_mission(42, m42_state)
            .expect("register pause");
        assert_eq!(m42_state, MissionState::Paused(PauseReason::Governor));

        // While still in Critical, cannot resume
        let _ = gov.on_health_update(HealthState::Critical);
        let res = gov.try_auto_resume_mission(42, m42_state);
        assert_eq!(res, None, "Cannot resume in Critical");

        // Health drops to Normal
        let _ = gov.on_health_update(HealthState::Normal);

        // Immediately after dropping to Normal, cooldown (30ms) is not yet met
        let res_immediate = gov.try_auto_resume_mission(42, m42_state);
        assert_eq!(res_immediate, None, "Cooldown not yet met");

        // Wait for cooldown
        std::thread::sleep(Duration::from_millis(35));

        // Now auto-resume succeeds!
        let res_after = gov.try_auto_resume_mission(42, m42_state);
        assert_eq!(res_after, Some(MissionState::Running));
        assert!(!gov.is_governor_paused(42));
    }

    #[test]
    fn test_governor_never_resumes_user_paused_or_approval_paused_missions() {
        let mut gov = Governor::new(2, Duration::from_millis(10));
        let _ = gov.on_health_update(HealthState::Normal);
        std::thread::sleep(Duration::from_millis(15));
        assert!(gov.can_auto_resume());

        // User paused mission
        let user_paused = MissionState::Paused(PauseReason::User);
        let res_user = gov.try_auto_resume_mission(99, user_paused);
        assert_eq!(
            res_user, None,
            "Governor must NEVER resume user-paused missions"
        );

        // Approval paused mission
        let approval_paused = MissionState::Paused(PauseReason::Approval);
        let res_approval = gov.try_auto_resume_mission(100, approval_paused);
        assert_eq!(
            res_approval, None,
            "Governor must NEVER resume approval-paused missions"
        );
    }

    #[test]
    #[allow(clippy::assert_is_empty)]
    fn test_simulated_load_scenario_pauses_and_resumes_mock_mission() {
        // Verification requirement for 14.7:
        // "Verify: stress-ng scenario (CPU + memory) drives state to Constrained/Critical;
        // a running mock mission pauses, then resumes after load stops."
        use starship_health::{
            CpuMetrics, HealthSample, MemoryMetrics, OverloadDetector, OverloadThresholds,
        };

        let thresholds = OverloadThresholds {
            enter_consecutive_samples: 1, // immediate trip for scenario testing
            exit_consecutive_samples: 2,  // 2 quiet samples to de-escalate
            cpu_critical_pct: 97.0,
            cpu_exit_pct: 70.0,
            ..Default::default()
        };

        let mut detector = OverloadDetector::new(thresholds);
        let mut gov = Governor::new(2, Duration::from_millis(40));

        let mut mission_state = MissionState::Running;
        let mission_id = 101;

        // Phase A: System is normal
        let normal_sample = HealthSample {
            cpu: Some(CpuMetrics {
                global_util_pct: 25.0,
                core_util_pct: vec![25.0],
                load_avg_1m: 0.5,
                load_avg_5m: 0.5,
                load_avg_15m: 0.5,
                avg_freq_mhz: 3000,
            }),
            memory: Some(MemoryMetrics {
                total_bytes: 16_000_000_000,
                used_bytes: 6_000_000_000,
                available_bytes: 10_000_000_000,
                available_pct: 62.5, // > 25% exit threshold
                swap_total_bytes: 4_000_000_000,
                swap_used_bytes: 0,
                swap_in_bytes_per_sec: 0.0,
            }),
            ..Default::default()
        };
        let (s_norm, _) = detector.update(&normal_sample);
        let d_norm = gov.on_health_update(s_norm);
        assert_eq!(d_norm.current_health_state, HealthState::Normal);
        assert!(!d_norm.should_pause);

        // Phase B: Heavy stress-ng simulation (CPU 99%, low mem)
        let stress_sample = HealthSample {
            cpu: Some(CpuMetrics {
                global_util_pct: 99.0,
                core_util_pct: vec![99.0],
                load_avg_1m: 16.0,
                load_avg_5m: 8.0,
                load_avg_15m: 4.0,
                avg_freq_mhz: 3600,
            }),
            memory: Some(MemoryMetrics {
                total_bytes: 16_000_000_000,
                used_bytes: 15_500_000_000,
                available_bytes: 500_000_000,
                available_pct: 3.125, // < 5% critical
                swap_total_bytes: 4_000_000_000,
                swap_used_bytes: 2_000_000_000,
                swap_in_bytes_per_sec: 0.0,
            }),
            ..Default::default()
        };

        let (s_crit, alerts) = detector.update(&stress_sample);
        assert_eq!(s_crit, HealthState::Critical);
        assert!(!alerts.is_empty());

        let d_crit = gov.on_health_update(s_crit);
        assert!(
            d_crit.should_pause,
            "Critical state directives MUST mandate safe-pause"
        );

        // Safe-pause the mission at step boundary
        mission_state = gov
            .register_paused_mission(mission_id, mission_state)
            .expect("pause mission");
        assert_eq!(mission_state, MissionState::Paused(PauseReason::Governor));

        // Phase C: Stress ends, system returns to normal
        // With exit_consecutive_samples = 2, stepping down from Critical -> Constrained -> Elevated -> Normal
        // requires 3 step-down windows of 2 samples each = 6 normal samples.
        for _ in 0..6 {
            let (state, _) = detector.update(&normal_sample);
            let _ = gov.on_health_update(state);
        }
        assert_eq!(detector.current_state(), HealthState::Normal);
        assert_eq!(gov.current_state(), HealthState::Normal);

        // Before cooldown, still paused
        let early_resume = gov.try_auto_resume_mission(mission_id, mission_state);
        assert_eq!(early_resume, None);

        // Wait for governor recovery window
        std::thread::sleep(Duration::from_millis(50));

        // Now auto-resume automatically triggers!
        let resumed_state = gov
            .try_auto_resume_mission(mission_id, mission_state)
            .expect("auto resume triggered");
        assert_eq!(resumed_state, MissionState::Running);
    }
}
