use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Minimum sample count before ETA is considered statistically significant (GAP-CLOSURE-PLAN.md §13.2).
pub const MIN_SAMPLES_FOR_PRECISE_ETA: usize = 5;

/// Minimum throttle interval: <= 4 Hz per mission (250ms).
pub const PROGRESS_THROTTLE_INTERVAL: Duration = Duration::from_millis(250);

/// Default stall timeout when no heartbeat is received: max(30s, 3 * `expected_interval`).
pub const DEFAULT_STALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Historical execution duration statistics for a specific tool (GAP-CLOSURE-PLAN.md §13.3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDurationStats {
    pub tool: String,
    pub n: usize,
    pub ewma_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
}

impl ToolDurationStats {
    #[must_use]
    pub fn new(tool: impl Into<String>, n: usize, ewma_ms: f64, p50_ms: f64, p90_ms: f64) -> Self {
        Self {
            tool: tool.into(),
            n,
            ewma_ms,
            p50_ms,
            p90_ms,
        }
    }
}

/// Estimated Time to Arrival (ETA) with uncertainty bounds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EtaEstimate {
    pub p50_ms: f64,
    pub p90_ms: f64,
    /// True when n < 5 samples, indicating a tilde `~` prefix in UI display.
    pub is_approximate: bool,
}

/// Per-step progress tracking record.
#[derive(Debug, Clone)]
pub struct StepProgressTracker {
    pub step_index: u32,
    pub tool_name: String,
    pub weight: f64,
    pub progress: f64,
    pub message: Option<String>,
    pub last_update: Instant,
    pub completed: bool,
}

impl StepProgressTracker {
    #[must_use]
    pub fn new(step_index: u32, tool_name: impl Into<String>, weight: f64) -> Self {
        Self {
            step_index,
            tool_name: tool_name.into(),
            weight: if weight <= 0.0 { 1.0 } else { weight },
            progress: 0.0,
            message: None,
            last_update: Instant::now(),
            completed: false,
        }
    }
}

/// Consolidated mission progress snapshot emitted to UI and event listeners.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MissionProgressSnapshot {
    pub mission_id: String,
    pub mission_progress: f64,
    pub active_step_index: Option<u32>,
    pub step_progress: f64,
    pub step_message: Option<String>,
    pub eta: Option<EtaEstimate>,
    pub is_stalled: bool,
    pub elapsed_ms: u64,
}

/// Central progress aggregator computing weighted progress, ETA, and stalls throttled to <= 4 Hz (GAP-CLOSURE-PLAN.md §13.2).
#[derive(Debug)]
pub struct ProgressAggregator {
    mission_id: String,
    steps: Vec<StepProgressTracker>,
    tool_history: HashMap<String, ToolDurationStats>,
    start_time: Instant,
    last_emitted: Option<Instant>,
    last_activity: Instant,
    expected_interval: Duration,
}

impl ProgressAggregator {
    /// Initializes a new progress aggregator for a mission with defined step weights and tool history.
    #[must_use]
    pub fn new(
        mission_id: impl Into<String>,
        steps: Vec<(u32, String, Option<f64>)>,
        tool_history: HashMap<String, ToolDurationStats>,
    ) -> Self {
        let now = Instant::now();
        let trackers = steps
            .into_iter()
            .map(|(idx, tool, explicit_weight)| {
                let weight = explicit_weight.unwrap_or_else(|| {
                    tool_history
                        .get(&tool)
                        .map_or(1.0, |stats| stats.p50_ms.max(1.0))
                });
                StepProgressTracker::new(idx, tool, weight)
            })
            .collect();

        Self {
            mission_id: mission_id.into(),
            steps: trackers,
            tool_history,
            start_time: now,
            last_emitted: None,
            last_activity: now,
            expected_interval: Duration::from_secs(5),
        }
    }

    /// Sets the expected progress update interval for stall detection.
    pub fn set_expected_interval(&mut self, interval: Duration) {
        self.expected_interval = interval;
    }

    /// Ingests a progress update for a specific step.
    pub fn update_step_progress(
        &mut self,
        step_index: u32,
        progress: f64,
        message: Option<String>,
    ) {
        let now = Instant::now();
        self.last_activity = now;

        if let Some(step) = self.steps.iter_mut().find(|s| s.step_index == step_index) {
            step.progress = progress.clamp(0.0, 1.0);
            step.last_update = now;
            if message.is_some() {
                step.message = message;
            }
            if (step.progress - 1.0).abs() < f64::EPSILON {
                step.completed = true;
            }
        }
    }

    /// Marks a step as completely finished (1.0).
    pub fn complete_step(&mut self, step_index: u32) {
        let now = Instant::now();
        self.last_activity = now;
        if let Some(step) = self.steps.iter_mut().find(|s| s.step_index == step_index) {
            step.progress = 1.0;
            step.completed = true;
            step.last_update = now;
        }
    }

    /// Records a heartbeat to keep stall detector alive during long-running silent tasks.
    pub fn heartbeat(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Computes overall weighted mission progress: `Σ(step.weight * step.progress) / Σ weights`.
    #[must_use]
    pub fn compute_mission_progress(&self) -> f64 {
        let total_weight: f64 = self.steps.iter().map(|s| s.weight).sum();
        if total_weight <= 0.0 {
            return 0.0;
        }

        let completed_weighted: f64 = self.steps.iter().map(|s| s.weight * s.progress).sum();
        (completed_weighted / total_weight).clamp(0.0, 1.0)
    }

    /// Computes remaining ETA bounds [p50, p90] based on remaining step weights and tool statistics.
    #[must_use]
    pub fn compute_eta(&self) -> Option<EtaEstimate> {
        let mut total_p50 = 0.0;
        let mut total_p90 = 0.0;
        let mut has_unestimated = false;
        let mut any_pending = false;

        for step in &self.steps {
            if step.completed || step.progress >= 1.0 {
                continue;
            }

            any_pending = true;
            let remaining_fraction = (1.0 - step.progress).max(0.0);

            if let Some(stats) = self.tool_history.get(&step.tool_name) {
                total_p50 += stats.p50_ms * remaining_fraction;
                total_p90 += stats.p90_ms * remaining_fraction;
                if stats.n < MIN_SAMPLES_FOR_PRECISE_ETA {
                    has_unestimated = true;
                }
            } else {
                // Fallback: estimate from step weight (or default 1000ms per unit weight)
                let fallback_ms = step.weight.max(1000.0);
                total_p50 += fallback_ms * remaining_fraction;
                total_p90 += fallback_ms * 1.5 * remaining_fraction;
                has_unestimated = true;
            }
        }

        if !any_pending {
            return Some(EtaEstimate {
                p50_ms: 0.0,
                p90_ms: 0.0,
                is_approximate: false,
            });
        }

        Some(EtaEstimate {
            p50_ms: total_p50,
            p90_ms: total_p90,
            is_approximate: has_unestimated,
        })
    }

    /// Checks if mission is stalled: `no progress or heartbeat for max(30s, 3 * expected_interval)`.
    #[must_use]
    pub fn is_stalled(&self) -> bool {
        let timeout = DEFAULT_STALL_TIMEOUT.max(self.expected_interval * 3);
        self.last_activity.elapsed() >= timeout
    }

    /// Evaluates throttle rule (<= 4 Hz / 250ms). If permitted, updates last emitted timestamp and returns snapshot.
    pub fn poll_snapshot(&mut self, force_flush: bool) -> Option<MissionProgressSnapshot> {
        let now = Instant::now();
        if !force_flush
            && let Some(last) = self.last_emitted
            && now.duration_since(last) < PROGRESS_THROTTLE_INTERVAL
        {
            return None;
        }

        self.last_emitted = Some(now);

        let active_step = self
            .steps
            .iter()
            .find(|s| !s.completed && s.progress < 1.0)
            .cloned();

        Some(MissionProgressSnapshot {
            mission_id: self.mission_id.clone(),
            mission_progress: self.compute_mission_progress(),
            active_step_index: active_step.as_ref().map(|s| s.step_index),
            step_progress: active_step.as_ref().map_or(1.0, |s| s.progress),
            step_message: active_step.and_then(|s| s.message),
            eta: self.compute_eta(),
            is_stalled: self.is_stalled(),
            #[allow(clippy::cast_possible_truncation)]
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_mission_progress() {
        // Step 0: weight 10.0 (e.g. 10s video render)
        // Step 1: weight 90.0 (e.g. 90s audio master)
        let steps = vec![
            (0, "render".to_string(), Some(10.0)),
            (1, "master".to_string(), Some(90.0)),
        ];

        let mut agg = ProgressAggregator::new("m1", steps, HashMap::new());
        assert_eq!(agg.compute_mission_progress(), 0.0);

        // Render is 100% done (contributes 10.0 / 100.0 = 0.10)
        agg.update_step_progress(0, 1.0, Some("Render done".into()));
        assert!((agg.compute_mission_progress() - 0.10).abs() < 1e-6);

        // Master is 50% done (contributes 45.0 / 100.0 = 0.45, total 0.55)
        agg.update_step_progress(1, 0.5, Some("Mastering audio".into()));
        assert!((agg.compute_mission_progress() - 0.55).abs() < 1e-6);

        // Master is 100% done
        agg.complete_step(1);
        assert!((agg.compute_mission_progress() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_eta_with_and_without_history() {
        let mut history = HashMap::new();
        // Tool with >= 5 samples (precise)
        history.insert(
            "known_tool".to_string(),
            ToolDurationStats::new("known_tool", 10, 5000.0, 4800.0, 5600.0),
        );
        // Tool with < 5 samples (approximate)
        history.insert(
            "sparse_tool".to_string(),
            ToolDurationStats::new("sparse_tool", 2, 2000.0, 1900.0, 2400.0),
        );

        // Scenario A: Only known tools with >= 5 samples
        let steps_precise = vec![(0, "known_tool".to_string(), None)];
        let agg_precise = ProgressAggregator::new("m_precise", steps_precise, history.clone());
        let eta_precise = agg_precise.compute_eta().expect("eta");
        assert_eq!(eta_precise.p50_ms, 4800.0);
        assert_eq!(eta_precise.p90_ms, 5600.0);
        assert!(!eta_precise.is_approximate);

        // Scenario B: Has tool with sparse samples (< 5)
        let steps_sparse = vec![
            (0, "known_tool".to_string(), None),
            (1, "sparse_tool".to_string(), None),
        ];
        let agg_sparse = ProgressAggregator::new("m_sparse", steps_sparse, history.clone());
        let eta_sparse = agg_sparse.compute_eta().expect("eta");
        assert_eq!(eta_sparse.p50_ms, 4800.0 + 1900.0);
        assert_eq!(eta_sparse.p90_ms, 5600.0 + 2400.0);
        assert!(eta_sparse.is_approximate);

        // Scenario C: Completely unknown tool (no history)
        let steps_unknown = vec![(0, "unknown_tool".to_string(), Some(3000.0))];
        let agg_unknown = ProgressAggregator::new("m_unknown", steps_unknown, history);
        let eta_unknown = agg_unknown.compute_eta().expect("eta");
        assert_eq!(eta_unknown.p50_ms, 3000.0);
        assert!(eta_unknown.is_approximate);
    }

    #[test]
    fn test_progress_throttling_at_4hz() {
        let steps = vec![(0, "tool".to_string(), Some(1.0))];
        let mut agg = ProgressAggregator::new("m_throttle", steps, HashMap::new());

        // First poll succeeds immediately
        let snap1 = agg.poll_snapshot(false);
        assert!(snap1.is_some());

        // Rapid subsequent poll without elapsed time is throttled (< 250ms)
        let snap2 = agg.poll_snapshot(false);
        assert!(snap2.is_none());

        // Force flush bypasses throttle
        let snap3 = agg.poll_snapshot(true);
        assert!(snap3.is_some());
    }

    #[test]
    fn test_stall_detection() {
        let steps = vec![(0, "tool".to_string(), Some(1.0))];
        let mut agg = ProgressAggregator::new("m_stall", steps, HashMap::new());
        agg.set_expected_interval(Duration::from_millis(50));

        assert!(!agg.is_stalled());

        // Fast-forward last_activity beyond timeout
        agg.last_activity = Instant::now()
            .checked_sub(Duration::from_secs(31))
            .expect("checked_sub");
        assert!(agg.is_stalled());

        // Heartbeat clears stall
        agg.heartbeat();
        assert!(!agg.is_stalled());
    }
}
