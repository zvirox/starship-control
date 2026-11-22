use crate::CoreError;
use crate::executor::lease::LeaseSupervisor;
use crate::executor::state::{
    MissionEvent, MissionState, StepEvent, StepState, transition_mission, transition_step,
};
use crate::safety::{ApprovalRequest, RiskTier, SafetyArbiter};
use serde::{Deserialize, Serialize};

/// Interrupted step needing human recovery resolution (GAP-CLOSURE-PLAN.md §11.9).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InterruptedStepInfo {
    pub step_id: i64,
    pub step_index: i64,
    pub app_id: String,
    pub tool_name: String,
    pub risk_tier: RiskTier,
    pub checkpoint_id: Option<String>,
}

/// Recovery card presented to the user on startup if an unclean crash occurred.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CrashRecoveryCard {
    pub mission_id: i64,
    pub plan_id: i64,
    pub mission_title: String,
    pub interrupted_steps: Vec<InterruptedStepInfo>,
}

/// Recovery action selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    Resume,
    RestartStep,
    Abort,
}

pub struct CrashRecoveryManager;

impl CrashRecoveryManager {
    /// Evaluates the outcome of a recovery action on an interrupted step (GAP-CLOSURE-PLAN.md §11.9).
    ///
    /// # Hard Invariants:
    /// - **Tier 2/3 steps NEVER auto-resume**. Resuming a Tier 2/3 step transitions it to `AwaitingApproval`
    ///   and issues a fresh argument-bound approval request.
    /// - Tier 1 safe steps can transition directly back to `Ready`.
    /// - Aborting cancels the step and transitions the mission to `Aborted`.
    pub fn resolve_step_recovery(
        step: &InterruptedStepInfo,
        action: RecoveryAction,
        arbiter: &SafetyArbiter,
        plan_id: i64,
        now_secs: u64,
    ) -> Result<(StepState, Option<ApprovalRequest>), CoreError> {
        match action {
            RecoveryAction::Resume => {
                // Invariant: Never auto-resume Tier 2/3 steps without fresh human approval
                if step.risk_tier == RiskTier::Mutating || step.risk_tier == RiskTier::Destructive {
                    let approval_req = arbiter.create_approval_request(
                        plan_id,
                        step.step_id,
                        &serde_json::json!({ "recovered": true, "tool": step.tool_name }),
                        step.risk_tier,
                        now_secs,
                    );
                    // Interrupted -> Ready -> RequireApproval -> AwaitingApproval
                    let ready =
                        transition_step(StepState::Interrupted, StepEvent::ResumeInterrupted)?;
                    let awaiting = transition_step(ready, StepEvent::RequireApproval)?;
                    Ok((awaiting, Some(approval_req)))
                } else {
                    // Safe Tier 1 step transitions cleanly back to Ready
                    let ready =
                        transition_step(StepState::Interrupted, StepEvent::ResumeInterrupted)?;
                    Ok((ready, None))
                }
            }
            RecoveryAction::RestartStep => {
                // Restarts step execution: resets to Ready
                let ready = transition_step(StepState::Interrupted, StepEvent::ResumeInterrupted)?;
                Ok((ready, None))
            }
            RecoveryAction::Abort => {
                // User cancels interrupted step
                let cancelled = transition_step(StepState::Interrupted, StepEvent::Cancel)?;
                Ok((cancelled, None))
            }
        }
    }

    /// Evaluates the mission-level state transition for crash recovery.
    pub fn resolve_mission_recovery(
        current: MissionState,
        action: RecoveryAction,
    ) -> Result<MissionState, CoreError> {
        match action {
            RecoveryAction::Resume | RecoveryAction::RestartStep => {
                // If mission was paused/interrupted during crash, resume to Running
                match current {
                    MissionState::Paused(_) => transition_mission(current, MissionEvent::Resume),
                    MissionState::Running => Ok(MissionState::Running),
                    _ => Ok(current),
                }
            }
            RecoveryAction::Abort => transition_mission(current, MissionEvent::Abort),
        }
    }

    /// Releases all stale leases associated with recovered missions.
    pub fn cleanup_stale_leases(supervisor: &LeaseSupervisor, mission_ids: &[i64]) -> usize {
        let mut total = 0;
        for &mid in mission_ids {
            total += supervisor.release_leases(mid, None);
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_2_and_3_steps_never_autoresume_and_require_fresh_approval() {
        let arbiter = SafetyArbiter::new("test-salt-secret");
        let now = 1_700_000_000;
        let plan_id = 100;

        // Tier 3 Destructive step interrupted by crash (e.g. kill -9)
        let tier3_step = InterruptedStepInfo {
            step_id: 301,
            step_index: 2,
            app_id: "atp".into(),
            tool_name: "purge_package_cache".into(),
            risk_tier: RiskTier::Destructive,
            checkpoint_id: None,
        };

        // Resume action requested
        let (state, approval_req) = CrashRecoveryManager::resolve_step_recovery(
            &tier3_step,
            RecoveryAction::Resume,
            &arbiter,
            plan_id,
            now,
        )
        .expect("resolve recovery");

        // Assert: Step state is strictly AwaitingApproval, NOT Running
        assert_eq!(state, StepState::AwaitingApproval);
        assert!(
            approval_req.is_some(),
            "Fresh approval request must be issued on resume"
        );
        let req = approval_req.unwrap();
        assert_eq!(req.risk_tier, RiskTier::Destructive);
        assert_eq!(req.step_id, 301);

        // Tier 1 Safe step can transition directly back to Ready
        let tier1_step = InterruptedStepInfo {
            step_id: 101,
            step_index: 0,
            app_id: "ssp".into(),
            tool_name: "screen.query_displays".into(),
            risk_tier: RiskTier::Safe,
            checkpoint_id: None,
        };

        let (state_t1, req_t1) = CrashRecoveryManager::resolve_step_recovery(
            &tier1_step,
            RecoveryAction::Resume,
            &arbiter,
            plan_id,
            now,
        )
        .expect("resolve tier 1 recovery");

        assert_eq!(state_t1, StepState::Ready);
        assert!(req_t1.is_none());

        // Abort action cleanly cancels
        let (state_aborted, _) = CrashRecoveryManager::resolve_step_recovery(
            &tier3_step,
            RecoveryAction::Abort,
            &arbiter,
            plan_id,
            now,
        )
        .expect("resolve abort");
        assert_eq!(state_aborted, StepState::Cancelled);
    }
}
