use crate::error::{CoreError, ErrorKind};
use crate::executor::failure::{FailureClass, classify_error};
use crate::executor::retry::RetryPolicy;
use crate::safety::RiskTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_FALLBACKS_TOML: &str = include_str!("fallbacks.toml");

/// Hierarchical ladder rungs per GAP-CLOSURE-PLAN.md §2.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackRung {
    /// F0 — Retry same call with backoff + jitter (only if allowed by §11.5).
    F0Retry,
    /// F1 — Alternate transport (e.g., UDS socket -> headless stdio MCP or restart app).
    F1AlternateTransport,
    /// F2 — Alternate tool/route from `fallbacks.toml` (same tier or higher only).
    F2AlternateTool,
    /// F3 — Alternate model / re-plan remaining DAG with failure context (max 1 re-plan per mission).
    F3Replan,
    /// F4 — Degrade: Skip steps flagged `optional: true`, mark partially succeeded.
    F4Degrade,
    /// F5 — Rollback: Restore checkpoints and quarantine files.
    F5Rollback,
    /// F6 — Safe stop & handoff: Cancel in-flight calls, release leases, write report.
    F6SafeStop,
}

/// Fallback decision recorded for every transition (GAP-CLOSURE-PLAN.md §2.4 Rule 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackDecision {
    pub from_rung: FallbackRung,
    pub to_rung: FallbackRung,
    pub reason: String,
    pub failure_class: FailureClass,
    pub target_tool: Option<String>,
    pub target_transport: Option<String>,
    pub requires_reapproval: bool,
    pub is_terminal: bool,
}

/// Dynamic execution context for an active step traversing the fallback ladder.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct StepExecutionState {
    pub step_id: String,
    pub tool_name: String,
    pub risk_tier: RiskTier,
    pub is_idempotent: bool,
    pub has_idempotency_key: bool,
    pub is_optional: bool,
    pub has_mutated: bool,
    pub current_rung: FallbackRung,
    pub f0_retry_count: u32,
    pub f1_transport_attempt: u32,
    pub f2_tool_index: usize,
    pub current_transport: String,
}

impl StepExecutionState {
    #[must_use]
    pub fn new(
        step_id: impl Into<String>,
        tool_name: impl Into<String>,
        risk_tier: RiskTier,
    ) -> Self {
        Self {
            step_id: step_id.into(),
            tool_name: tool_name.into(),
            risk_tier,
            is_idempotent: false,
            has_idempotency_key: false,
            is_optional: false,
            has_mutated: false,
            current_rung: FallbackRung::F0Retry,
            f0_retry_count: 0,
            f1_transport_attempt: 0,
            f2_tool_index: 0,
            current_transport: "uds_socket".to_string(),
        }
    }
}

/// Mission-wide fallback tracking context.
#[derive(Debug, Clone)]
pub struct MissionFallbackContext {
    pub mission_id: i64,
    pub replan_count: u32,
    pub max_replans: u32,
}

impl MissionFallbackContext {
    #[must_use]
    pub fn new(mission_id: i64) -> Self {
        Self {
            mission_id,
            replan_count: 0,
            max_replans: 1, // Max 1 re-plan per mission (§2.2 F3)
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ToolFallbackEntry {
    alternatives: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct FallbacksTomlConfig {
    #[serde(default)]
    tools: HashMap<String, ToolFallbackEntry>,
}

/// Fallback Ladder Engine executing §2.1–§2.4 of GAP-CLOSURE-PLAN.md.
#[derive(Debug, Clone)]
pub struct FallbackLadderEngine {
    tool_alternatives: HashMap<String, Vec<String>>,
    retry_policy: RetryPolicy,
}

impl Default for FallbackLadderEngine {
    fn default() -> Self {
        Self::from_toml(DEFAULT_FALLBACKS_TOML)
            .expect("embedded fallbacks.toml configuration is valid")
    }
}

impl FallbackLadderEngine {
    /// Loads fallback mappings from a TOML configuration string.
    pub fn from_toml(content: &str) -> Result<Self, CoreError> {
        let config: FallbacksTomlConfig = toml::from_str(content).map_err(|err| {
            CoreError::new(
                ErrorKind::Settings,
                "settings.invalid_config",
                false,
                format!("failed to parse fallbacks.toml: {err}"),
                None,
            )
        })?;

        let mut alternatives = HashMap::new();
        for (tool, entry) in config.tools {
            alternatives.insert(tool, entry.alternatives);
        }

        Ok(Self {
            tool_alternatives: alternatives,
            retry_policy: RetryPolicy::default(),
        })
    }

    /// Evaluates the next ladder rung for a failed step according to the failure taxonomy and hard rules.
    pub fn next_action(
        &self,
        mission: &mut MissionFallbackContext,
        step: &mut StepExecutionState,
        error_code: &str,
    ) -> FallbackDecision {
        let failure_class = classify_error(error_code);

        // Rule 2.1: Safety failures NEVER retry, never F1/F2/F3. Rollback if mutated, else safe stop.
        if failure_class == FailureClass::Safety {
            return Self::handle_safety_or_user_failure(
                step,
                failure_class,
                "Safety policy violation",
            );
        }

        // Rule 2.1: User abort/rejection -> Stop immediately, rollback if partial mutations occurred.
        if failure_class == FailureClass::User {
            return Self::handle_safety_or_user_failure(
                step,
                failure_class,
                "User cancelled or rejected approval",
            );
        }

        // Rule 2.1: Resource exhaustion (VRAM OOM, disk space) -> skip F0/F1, jump to F3 or Pause/Rollback.
        if failure_class == FailureClass::Resource && step.current_rung < FallbackRung::F3Replan {
            step.current_rung = FallbackRung::F3Replan;
        }

        // Traverse the ladder based on current rung
        match step.current_rung {
            FallbackRung::F0Retry => {
                let retry_decision = self.retry_policy.evaluate_retry(
                    step.risk_tier,
                    failure_class,
                    step.is_idempotent,
                    step.has_idempotency_key,
                    step.f0_retry_count,
                );

                if retry_decision.should_retry {
                    step.f0_retry_count += 1;
                    FallbackDecision {
                        from_rung: FallbackRung::F0Retry,
                        to_rung: FallbackRung::F0Retry,
                        reason: format!(
                            "Retrying step {} (attempt {} of {}) with {} ms backoff",
                            step.step_id,
                            step.f0_retry_count,
                            retry_decision.max_retries,
                            retry_decision.backoff_ms
                        ),
                        failure_class,
                        target_tool: None,
                        target_transport: None,
                        requires_reapproval: false,
                        is_terminal: false,
                    }
                } else {
                    // F0 exhausted or prohibited -> Advance to F1
                    self.advance_to_f1_transport(step, failure_class)
                }
            }
            FallbackRung::F1AlternateTransport => {
                // If F1 also failed, advance to F2
                self.advance_to_f2_tool(step, failure_class)
            }
            FallbackRung::F2AlternateTool => {
                // If F2 also failed, advance to F3
                Self::advance_to_f3_replan(mission, step, failure_class)
            }
            FallbackRung::F3Replan => {
                // If F3 re-plan exhausted, advance to F4 (degrade) or F5 (rollback)
                Self::advance_to_f4_or_f5(step, failure_class)
            }
            FallbackRung::F4Degrade => {
                // If already degraded, nothing further to skip -> F5 or F6
                Self::advance_to_f5_or_f6(step, failure_class)
            }
            FallbackRung::F5Rollback => {
                // Rollback finished -> F6 Safe Stop
                Self::advance_to_f6_safe_stop(step, failure_class)
            }
            FallbackRung::F6SafeStop => FallbackDecision {
                from_rung: FallbackRung::F6SafeStop,
                to_rung: FallbackRung::F6SafeStop,
                reason: format!("Terminal safe stop reached for step {}", step.step_id),
                failure_class,
                target_tool: None,
                target_transport: None,
                requires_reapproval: false,
                is_terminal: true,
            },
        }
    }

    fn handle_safety_or_user_failure(
        step: &mut StepExecutionState,
        failure_class: FailureClass,
        detail: &str,
    ) -> FallbackDecision {
        let from = step.current_rung;
        if step.has_mutated {
            step.current_rung = FallbackRung::F5Rollback;
            FallbackDecision {
                from_rung: from,
                to_rung: FallbackRung::F5Rollback,
                reason: format!("{detail}: initiating rollback of partial mutations"),
                failure_class,
                target_tool: None,
                target_transport: None,
                requires_reapproval: false,
                is_terminal: false,
            }
        } else {
            step.current_rung = FallbackRung::F6SafeStop;
            FallbackDecision {
                from_rung: from,
                to_rung: FallbackRung::F6SafeStop,
                reason: format!("{detail}: stopping execution cleanly with no mutations"),
                failure_class,
                target_tool: None,
                target_transport: None,
                requires_reapproval: false,
                is_terminal: true,
            }
        }
    }

    fn advance_to_f1_transport(
        &self,
        step: &mut StepExecutionState,
        failure_class: FailureClass,
    ) -> FallbackDecision {
        if step.f1_transport_attempt == 0 {
            step.f1_transport_attempt += 1;
            let from = step.current_rung;
            step.current_rung = FallbackRung::F1AlternateTransport;

            let next_transport = if step.current_transport == "uds_socket" {
                "stdio_mcp"
            } else {
                "uds_socket"
            };
            step.current_transport = next_transport.to_string();

            FallbackDecision {
                from_rung: from,
                to_rung: FallbackRung::F1AlternateTransport,
                reason: format!("Transport failed. Switching transport to {next_transport}"),
                failure_class,
                target_tool: None,
                target_transport: Some(next_transport.to_string()),
                requires_reapproval: false,
                is_terminal: false,
            }
        } else {
            self.advance_to_f2_tool(step, failure_class)
        }
    }

    fn advance_to_f2_tool(
        &self,
        step: &mut StepExecutionState,
        failure_class: FailureClass,
    ) -> FallbackDecision {
        if let Some(alternatives) = self.tool_alternatives.get(&step.tool_name)
            && step.f2_tool_index < alternatives.len()
        {
            let alt_tool = alternatives[step.f2_tool_index].clone();
            step.f2_tool_index += 1;
            let from = step.current_rung;
            step.current_rung = FallbackRung::F2AlternateTool;

            // Rule 2.4: Fallbacks never auto-approve or reduce tier.
            // Mutating and destructive tools require approval for the alternative.
            let requires_reapproval = step.risk_tier >= RiskTier::Mutating;

            return FallbackDecision {
                from_rung: from,
                to_rung: FallbackRung::F2AlternateTool,
                reason: format!("Trying alternative tool {alt_tool} for {}", step.tool_name),
                failure_class,
                target_tool: Some(alt_tool),
                target_transport: None,
                requires_reapproval,
                is_terminal: false,
            };
        }

        // No alternatives left -> Advance to F3
        Self::advance_to_f3_replan(&mut MissionFallbackContext::new(0), step, failure_class)
    }

    fn advance_to_f3_replan(
        mission: &mut MissionFallbackContext,
        step: &mut StepExecutionState,
        failure_class: FailureClass,
    ) -> FallbackDecision {
        if mission.replan_count < mission.max_replans {
            mission.replan_count += 1;
            let from = step.current_rung;
            step.current_rung = FallbackRung::F3Replan;

            // Rule 2.4: A re-plan is a new plan version; approvals do not carry over.
            FallbackDecision {
                from_rung: from,
                to_rung: FallbackRung::F3Replan,
                reason: format!(
                    "Alternate tool/transport failed. Triggering DAG re-plan (attempt {} of {})",
                    mission.replan_count, mission.max_replans
                ),
                failure_class,
                target_tool: None,
                target_transport: None,
                requires_reapproval: true,
                is_terminal: false,
            }
        } else {
            Self::advance_to_f4_or_f5(step, failure_class)
        }
    }

    fn advance_to_f4_or_f5(
        step: &mut StepExecutionState,
        failure_class: FailureClass,
    ) -> FallbackDecision {
        if step.is_optional {
            let from = step.current_rung;
            step.current_rung = FallbackRung::F4Degrade;
            FallbackDecision {
                from_rung: from,
                to_rung: FallbackRung::F4Degrade,
                reason: format!(
                    "Step {} is marked optional; skipping step and continuing in degraded mode",
                    step.step_id
                ),
                failure_class,
                target_tool: None,
                target_transport: None,
                requires_reapproval: false,
                is_terminal: false,
            }
        } else {
            Self::advance_to_f5_or_f6(step, failure_class)
        }
    }

    fn advance_to_f5_or_f6(
        step: &mut StepExecutionState,
        failure_class: FailureClass,
    ) -> FallbackDecision {
        if step.has_mutated {
            let from = step.current_rung;
            step.current_rung = FallbackRung::F5Rollback;
            FallbackDecision {
                from_rung: from,
                to_rung: FallbackRung::F5Rollback,
                reason: format!(
                    "Required step {} failed; initiating rollback of mutations via checkpoints/quarantine",
                    step.step_id
                ),
                failure_class,
                target_tool: None,
                target_transport: None,
                requires_reapproval: false,
                is_terminal: false,
            }
        } else {
            Self::advance_to_f6_safe_stop(step, failure_class)
        }
    }

    fn advance_to_f6_safe_stop(
        step: &mut StepExecutionState,
        failure_class: FailureClass,
    ) -> FallbackDecision {
        let from = step.current_rung;
        step.current_rung = FallbackRung::F6SafeStop;
        FallbackDecision {
            from_rung: from,
            to_rung: FallbackRung::F6SafeStop,
            reason: format!(
                "Execution halted safely for step {}; releasing locks and awaiting user intervention",
                step.step_id
            ),
            failure_class,
            target_tool: None,
            target_transport: None,
            requires_reapproval: false,
            is_terminal: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ladder_rungs_full_progression_from_f0_to_f6() {
        let engine = FallbackLadderEngine::default();
        let mut mission = MissionFallbackContext::new(101);
        let mut step = StepExecutionState::new("step-cut", "eep.cut_batch", RiskTier::Mutating);
        step.is_idempotent = true;
        step.has_mutated = true;
        step.is_optional = false;

        // F0: Retry 1
        let d1 = engine.next_action(&mut mission, &mut step, "mcp.execution_failed");
        assert_eq!(d1.from_rung, FallbackRung::F0Retry);
        assert_eq!(d1.to_rung, FallbackRung::F0Retry);
        assert_eq!(step.f0_retry_count, 1);

        // F0 retries exhausted (AppFailure max 1 retry for idempotent) -> Advance to F1
        let d2 = engine.next_action(&mut mission, &mut step, "mcp.execution_failed");
        assert_eq!(d2.from_rung, FallbackRung::F0Retry);
        assert_eq!(d2.to_rung, FallbackRung::F1AlternateTransport);
        assert_eq!(d2.target_transport.as_deref(), Some("stdio_mcp"));

        // F1 fails -> Advance to F2 (Alternate Tool)
        let d3 = engine.next_action(&mut mission, &mut step, "mcp.execution_failed");
        assert_eq!(d3.from_rung, FallbackRung::F1AlternateTransport);
        assert_eq!(d3.to_rung, FallbackRung::F2AlternateTool);
        assert_eq!(d3.target_tool.as_deref(), Some("eep.cut_interval"));
        assert!(
            d3.requires_reapproval,
            "Tier 2 alternative tool requires re-approval"
        );

        // F2 fails (no more alternatives) -> Advance to F3 (Re-plan)
        let d4 = engine.next_action(&mut mission, &mut step, "mcp.execution_failed");
        assert_eq!(d4.from_rung, FallbackRung::F2AlternateTool);
        assert_eq!(d4.to_rung, FallbackRung::F3Replan);
        assert_eq!(mission.replan_count, 1);
        assert!(d4.requires_reapproval);

        // F3 fails (re-plan exhausted, step not optional, has_mutated = true) -> Advance to F5 (Rollback)
        let d5 = engine.next_action(&mut mission, &mut step, "mcp.execution_failed");
        assert_eq!(d5.from_rung, FallbackRung::F3Replan);
        assert_eq!(d5.to_rung, FallbackRung::F5Rollback);

        // F5 rollback complete -> Advance to F6 (Safe Stop)
        let d6 = engine.next_action(&mut mission, &mut step, "mcp.execution_failed");
        assert_eq!(d6.from_rung, FallbackRung::F5Rollback);
        assert_eq!(d6.to_rung, FallbackRung::F6SafeStop);
        assert!(d6.is_terminal);
    }

    #[test]
    fn test_f4_degrade_for_optional_step() {
        let engine = FallbackLadderEngine::default();
        let mut mission = MissionFallbackContext::new(102);
        mission.replan_count = 1; // Re-plans already exhausted

        let mut step = StepExecutionState::new("step-intro", "alp.intro_voiceover", RiskTier::Safe);
        step.current_rung = FallbackRung::F3Replan;
        step.is_optional = true;

        let decision = engine.next_action(&mut mission, &mut step, "mcp.execution_failed");
        assert_eq!(decision.from_rung, FallbackRung::F3Replan);
        assert_eq!(decision.to_rung, FallbackRung::F4Degrade);
        assert!(!decision.is_terminal);
    }

    #[test]
    fn test_safety_violation_never_retries_and_rolls_back_if_mutated() {
        let engine = FallbackLadderEngine::default();
        let mut mission = MissionFallbackContext::new(103);
        let mut step =
            StepExecutionState::new("step-delete", "atp.wipe_cache", RiskTier::Destructive);
        step.has_mutated = true;

        let decision = engine.next_action(&mut mission, &mut step, "safety.tier_violation");
        assert_eq!(decision.to_rung, FallbackRung::F5Rollback);
        assert_eq!(decision.failure_class, FailureClass::Safety);
    }

    #[test]
    fn test_safety_violation_stops_immediately_if_unmutated() {
        let engine = FallbackLadderEngine::default();
        let mut mission = MissionFallbackContext::new(104);
        let mut step =
            StepExecutionState::new("step-delete", "atp.wipe_cache", RiskTier::Destructive);
        step.has_mutated = false;

        let decision = engine.next_action(&mut mission, &mut step, "safety.approval_rejected");
        assert_eq!(decision.to_rung, FallbackRung::F6SafeStop);
        assert!(decision.is_terminal);
    }

    #[test]
    fn test_tier_3_step_never_auto_retries_at_f0() {
        let engine = FallbackLadderEngine::default();
        let mut mission = MissionFallbackContext::new(105);
        let mut step =
            StepExecutionState::new("step-wipe", "atp.clean_cache_all", RiskTier::Destructive);

        // Tier 3 with transient error -> F0 is skipped directly to F1
        let decision = engine.next_action(&mut mission, &mut step, "mcp.socket_unreachable");
        assert_eq!(decision.from_rung, FallbackRung::F0Retry);
        assert_eq!(decision.to_rung, FallbackRung::F1AlternateTransport);
    }
}
