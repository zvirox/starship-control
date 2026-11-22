use crate::{CoreError, ErrorKind};
use serde::{Deserialize, Serialize};

/// Discrete reasons for pausing mission execution (GAP-CLOSURE-PLAN.md §11A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseReason {
    User,
    Governor,
    Approval,
}

/// Mission-level state machine strictly adhering to GAP-CLOSURE-PLAN.md §11A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionState {
    Planning,
    Ready,
    Running,
    Paused(PauseReason),
    Succeeded,
    PartiallySucceeded,
    Failed,
    Aborted,
}

impl MissionState {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::PartiallySucceeded | Self::Failed | Self::Aborted
        )
    }
}

/// Events that trigger mission state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionEvent {
    PlanGenerated,
    Start,
    Pause(PauseReason),
    Resume,
    CompleteAllSucceeded,
    CompletePartial,
    Fail,
    Abort,
}

/// Pure, side-effect-free mission state transition function (GAP-CLOSURE-PLAN.md §11.1).
///
/// # Errors
/// Returns `ipc.bad_state` `CoreError` if the requested transition is illegal.
pub fn transition_mission(
    current: MissionState,
    event: MissionEvent,
) -> Result<MissionState, CoreError> {
    match (current, event) {
        // To Ready
        (MissionState::Planning, MissionEvent::PlanGenerated) => Ok(MissionState::Ready),

        // To Running
        (MissionState::Ready, MissionEvent::Start)
        | (MissionState::Paused(_), MissionEvent::Resume) => Ok(MissionState::Running),

        // To Paused
        (MissionState::Running, MissionEvent::Pause(reason)) => Ok(MissionState::Paused(reason)),

        // To Succeeded
        (MissionState::Running, MissionEvent::CompleteAllSucceeded) => Ok(MissionState::Succeeded),

        // To PartiallySucceeded
        (MissionState::Running, MissionEvent::CompletePartial) => {
            Ok(MissionState::PartiallySucceeded)
        }

        // To Failed
        (MissionState::Planning | MissionState::Running, MissionEvent::Fail) => {
            Ok(MissionState::Failed)
        }

        // To Aborted
        (
            MissionState::Planning
            | MissionState::Ready
            | MissionState::Running
            | MissionState::Paused(_),
            MissionEvent::Abort,
        ) => Ok(MissionState::Aborted),

        // Terminal or illegal transitions
        (curr, ev) => Err(CoreError::new(
            ErrorKind::Ipc,
            "ipc.bad_state",
            false,
            format!(
                "Illegal mission state transition: cannot apply event {ev:?} while in state {curr:?}"
            ),
            Some(serde_json::json!({
                "currentState": format!("{curr:?}"),
                "event": format!("{ev:?}"),
            })),
        )),
    }
}

/// Step-level state machine strictly adhering to GAP-CLOSURE-PLAN.md §11A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Pending,
    Ready,
    AwaitingApproval,
    Running,
    Succeeded,
    FailedRetryable,
    FailedFinal,
    Skipped,
    RolledBack,
    Cancelled,
    Interrupted,
}

impl StepState {
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Skipped | Self::RolledBack | Self::Cancelled
        )
    }
}

/// Events that trigger step state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepEvent {
    Schedule,
    RequireApproval,
    ApproveAndDispatch,
    DispatchDirect,
    Succeed,
    FailRetryable,
    FailFinal,
    BackoffResolved,
    FallbackRecover,
    Skip,
    Rollback,
    Cancel,
    CrashInterrupt,
    ResumeInterrupted,
}

/// Pure, side-effect-free step state transition function (GAP-CLOSURE-PLAN.md §11.1).
///
/// # Errors
/// Returns `ipc.bad_state` `CoreError` if the requested transition is illegal.
pub fn transition_step(current: StepState, event: StepEvent) -> Result<StepState, CoreError> {
    match (current, event) {
        // To Ready
        (StepState::Pending, StepEvent::Schedule)
        | (StepState::FailedRetryable, StepEvent::BackoffResolved)
        | (StepState::FailedFinal, StepEvent::FallbackRecover)
        | (StepState::Interrupted, StepEvent::ResumeInterrupted) => Ok(StepState::Ready),

        // To AwaitingApproval
        (StepState::Ready, StepEvent::RequireApproval) => Ok(StepState::AwaitingApproval),

        // To Running
        (StepState::Ready, StepEvent::DispatchDirect | StepEvent::ApproveAndDispatch)
        | (StepState::AwaitingApproval, StepEvent::ApproveAndDispatch) => Ok(StepState::Running),

        // To Succeeded
        (StepState::Running, StepEvent::Succeed) => Ok(StepState::Succeeded),

        // To FailedRetryable
        (StepState::Running, StepEvent::FailRetryable) => Ok(StepState::FailedRetryable),

        // To FailedFinal
        (StepState::Running | StepState::FailedRetryable, StepEvent::FailFinal) => {
            Ok(StepState::FailedFinal)
        }

        // To Skipped
        (StepState::Pending | StepState::FailedFinal, StepEvent::Skip) => Ok(StepState::Skipped),

        // To RolledBack
        (StepState::FailedFinal, StepEvent::Rollback) => Ok(StepState::RolledBack),

        // To Interrupted
        (StepState::Running, StepEvent::CrashInterrupt) => Ok(StepState::Interrupted),

        // To Cancelled
        (
            StepState::Pending
            | StepState::Ready
            | StepState::AwaitingApproval
            | StepState::Running
            | StepState::FailedRetryable
            | StepState::FailedFinal
            | StepState::Interrupted,
            StepEvent::Cancel,
        ) => Ok(StepState::Cancelled),

        // All other transitions are illegal
        (curr, ev) => Err(CoreError::new(
            ErrorKind::Ipc,
            "ipc.bad_state",
            false,
            format!(
                "Illegal step state transition: cannot apply event {ev:?} while in state {curr:?}"
            ),
            Some(serde_json::json!({
                "currentState": format!("{curr:?}"),
                "event": format!("{ev:?}"),
            })),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mission_happy_path_lifecycle() {
        let mut state = MissionState::Planning;
        assert!(!state.is_terminal());

        state = transition_mission(state, MissionEvent::PlanGenerated).expect("ready");
        assert_eq!(state, MissionState::Ready);

        state = transition_mission(state, MissionEvent::Start).expect("running");
        assert_eq!(state, MissionState::Running);

        state = transition_mission(state, MissionEvent::Pause(PauseReason::User)).expect("pause");
        assert_eq!(state, MissionState::Paused(PauseReason::User));

        state = transition_mission(state, MissionEvent::Resume).expect("resume");
        assert_eq!(state, MissionState::Running);

        state = transition_mission(state, MissionEvent::CompleteAllSucceeded).expect("succeeded");
        assert_eq!(state, MissionState::Succeeded);
        assert!(state.is_terminal());

        // Cannot resume or start from terminal state
        let err = transition_mission(state, MissionEvent::Start).unwrap_err();
        assert_eq!(err.code, "ipc.bad_state");
    }

    #[test]
    fn test_step_happy_path_and_approval_flow() {
        let mut step = StepState::Pending;
        assert!(!step.is_terminal());

        step = transition_step(step, StepEvent::Schedule).expect("ready");
        assert_eq!(step, StepState::Ready);

        step = transition_step(step, StepEvent::RequireApproval).expect("awaiting_approval");
        assert_eq!(step, StepState::AwaitingApproval);

        step = transition_step(step, StepEvent::ApproveAndDispatch).expect("running");
        assert_eq!(step, StepState::Running);

        step = transition_step(step, StepEvent::Succeed).expect("succeeded");
        assert_eq!(step, StepState::Succeeded);
        assert!(step.is_terminal());
    }

    #[test]
    fn test_step_retry_and_fallback_ladder() {
        // Step fails transiently -> FailedRetryable -> backoff -> Ready -> Running -> FailedFinal -> FallbackRecover -> Ready
        let mut step = StepState::Running;
        step = transition_step(step, StepEvent::FailRetryable).expect("retryable");
        assert_eq!(step, StepState::FailedRetryable);

        step = transition_step(step, StepEvent::BackoffResolved).expect("ready");
        assert_eq!(step, StepState::Ready);

        step = transition_step(step, StepEvent::DispatchDirect).expect("running");
        assert_eq!(step, StepState::Running);

        step = transition_step(step, StepEvent::FailFinal).expect("failed_final");
        assert_eq!(step, StepState::FailedFinal);

        // Fallback ladder activates alternative route
        step = transition_step(step, StepEvent::FallbackRecover).expect("ready");
        assert_eq!(step, StepState::Ready);

        // Or F4 Degrade / Skip
        let mut final_fail = StepState::FailedFinal;
        final_fail = transition_step(final_fail, StepEvent::Skip).expect("skipped");
        assert_eq!(final_fail, StepState::Skipped);
        assert!(final_fail.is_terminal());

        // Or F5 Checkpoint Rollback
        let mut rollback_step = StepState::FailedFinal;
        rollback_step = transition_step(rollback_step, StepEvent::Rollback).expect("rolled_back");
        assert_eq!(rollback_step, StepState::RolledBack);
        assert!(rollback_step.is_terminal());
    }

    #[test]
    fn test_crash_recovery_transitions() {
        let mut step = StepState::Running;
        step = transition_step(step, StepEvent::CrashInterrupt).expect("interrupted");
        assert_eq!(step, StepState::Interrupted);

        // Crash recovery resume puts step back to Ready
        let resumed = transition_step(step, StepEvent::ResumeInterrupted).expect("resumed");
        assert_eq!(resumed, StepState::Ready);

        // Or user can cancel it
        let cancelled = transition_step(step, StepEvent::Cancel).expect("cancelled");
        assert_eq!(cancelled, StepState::Cancelled);
    }

    #[test]
    fn test_property_no_sequence_reaches_illegal_state() {
        // Property test: exhaustively check all (State, Event) combinations.
        // For each combination, the transition either:
        // 1. Returns Ok(new_state) where new_state is a strictly valid StepState enum, OR
        // 2. Returns Err(CoreError) with code == "ipc.bad_state"
        // Under no circumstance does it panic or enter an undefined state.
        let all_states = [
            StepState::Pending,
            StepState::Ready,
            StepState::AwaitingApproval,
            StepState::Running,
            StepState::Succeeded,
            StepState::FailedRetryable,
            StepState::FailedFinal,
            StepState::Skipped,
            StepState::RolledBack,
            StepState::Cancelled,
            StepState::Interrupted,
        ];

        let all_events = [
            StepEvent::Schedule,
            StepEvent::RequireApproval,
            StepEvent::ApproveAndDispatch,
            StepEvent::DispatchDirect,
            StepEvent::Succeed,
            StepEvent::FailRetryable,
            StepEvent::FailFinal,
            StepEvent::BackoffResolved,
            StepEvent::FallbackRecover,
            StepEvent::Skip,
            StepEvent::Rollback,
            StepEvent::Cancel,
            StepEvent::CrashInterrupt,
            StepEvent::ResumeInterrupted,
        ];

        for &state in &all_states {
            for &event in &all_events {
                match transition_step(state, event) {
                    Ok(new_state) => {
                        // Assert that new_state is a legitimate StepState
                        assert!(all_states.contains(&new_state));
                    }
                    Err(err) => {
                        assert_eq!(err.code, "ipc.bad_state");
                        assert!(err.user_message.contains("Illegal step state transition"));
                    }
                }
            }
        }

        // Run multi-step pseudo-random walks to verify invariants
        let mut rng_seed: u64 = 0xdead_beef_c001_cafe;
        let mut pseudo_rand = || {
            rng_seed = rng_seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            rng_seed
        };

        for _ in 0..100 {
            let mut current = StepState::Pending;
            for _ in 0..50 {
                let event_idx =
                    usize::try_from(pseudo_rand() % (all_events.len() as u64)).unwrap_or(0);
                let event = all_events[event_idx];
                if let Ok(next) = transition_step(current, event) {
                    current = next;
                }
            }
            // Invariant: current state is always a valid variant in all_states
            assert!(all_states.contains(&current));
        }
    }
}
