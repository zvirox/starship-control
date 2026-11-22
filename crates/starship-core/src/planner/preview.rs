use crate::error::CoreError;
use crate::planner::tasksheet::TaskSheet;
use crate::planner::validator::TaskSheetValidator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Estimated resource profile required to execute a plan (GAP-CLOSURE-PLAN.md §12.5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProfileEstimate {
    pub peak_vram_mb: u64,
    pub peak_cpu_pct: f32,
    pub disk_space_required_bytes: u64,
}

/// Comprehensive plan preview showing DAG tiers, checkpoints, duration, and resource profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanPreview {
    pub plan_goal: String,
    pub total_steps: usize,
    pub tier_counts: HashMap<u8, usize>,
    pub estimated_duration_ms: u64,
    pub resource_profile: ResourceProfileEstimate,
    pub required_checkpoints: Vec<String>,
    pub requires_human_approval: bool,
}

impl PlanPreview {
    /// Generates a preview for a validated `TaskSheet` plan.
    #[must_use]
    pub fn from_tasksheet(plan: &TaskSheet) -> Self {
        let mut tier_counts = HashMap::new();
        let mut required_checkpoints = Vec::new();
        let mut estimated_duration_ms: u64 = 0;

        for step in &plan.steps {
            *tier_counts.entry(step.risk_tier).or_insert(0) += 1;

            // Checkpoints are mandatory before the first Tier 2+ mutation touching a resource
            if step.risk_tier >= 2 {
                required_checkpoints.push(format!(
                    "chk:{}:{}:{}",
                    step.app_id, step.step_index, step.tool_name
                ));
            }

            // Estimate step durations: Tier 1: ~1000ms, Tier 2: ~3000ms, Tier 3: ~5000ms
            let step_dur = match step.risk_tier {
                1 => 1000,
                2 => 3000,
                3 => 5000,
                _ => 1500,
            };
            estimated_duration_ms += step_dur;
        }

        let requires_human_approval = plan.steps.iter().any(|s| s.risk_tier >= 2);

        // Compute resource profile estimates
        let peak_vram_mb = if plan
            .steps
            .iter()
            .any(|s| s.tool_name.contains("denoise") || s.tool_name.contains("auto"))
        {
            1024
        } else {
            256
        };

        let peak_cpu_pct = if plan.steps.len() > 5 { 65.0 } else { 30.0 };
        let disk_space_required_bytes = (plan.steps.len() as u64) * 10 * 1024 * 1024; // ~10 MB per step scratch

        Self {
            plan_goal: plan.goal.clone(),
            total_steps: plan.steps.len(),
            tier_counts,
            estimated_duration_ms,
            resource_profile: ResourceProfileEstimate {
                peak_vram_mb,
                peak_cpu_pct,
                disk_space_required_bytes,
            },
            required_checkpoints,
            requires_human_approval,
        }
    }
}

/// Result of evaluating an individual step during a dry run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DryRunStepResult {
    pub step_index: u32,
    pub tool_name: String,
    pub risk_tier: u8,
    pub simulated: bool,
    pub success: bool,
    pub message: String,
}

/// Summary report returned by `plan_dry_run` (GAP-CLOSURE-PLAN.md §12.5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DryRunReport {
    pub total_steps: usize,
    pub tier1_executed: usize,
    pub tier2_simulated: usize,
    pub tier3_simulated: usize,
    pub step_results: Vec<DryRunStepResult>,
    pub all_passed: bool,
}

/// Handler for executing Tier 1 inspection tools during a dry run.
pub trait Tier1DryRunCaller {
    fn call_tier1_tool(
        &mut self,
        app_id: &str,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<(), CoreError>;
}

impl<F> Tier1DryRunCaller for F
where
    F: FnMut(&str, &str, &serde_json::Value) -> Result<(), CoreError>,
{
    fn call_tier1_tool(
        &mut self,
        app_id: &str,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<(), CoreError> {
        self(app_id, tool_name, args)
    }
}

/// Dry-run executor strictly gating Tier 2 and Tier 3 tools from execution (GAP-CLOSURE-PLAN.md §12.5).
pub struct DryRunExecutor;

impl DryRunExecutor {
    /// Executes a dry run: Tier 1 tools execute live inspection; Tier 2/3 tools are strictly simulated and NEVER called.
    pub fn execute<C: Tier1DryRunCaller>(
        plan: &TaskSheet,
        validator: &TaskSheetValidator,
        caller: &mut C,
    ) -> Result<DryRunReport, CoreError> {
        // Validate plan integrity first
        validator.validate(plan)?;

        let mut tier1_executed = 0;
        let mut tier2_simulated = 0;
        let mut tier3_simulated = 0;
        let mut step_results = Vec::with_capacity(plan.steps.len());
        let mut all_passed = true;

        for step in &plan.steps {
            match step.risk_tier {
                1 => {
                    // Tier 1: Execute live inspection
                    match caller.call_tier1_tool(&step.app_id, &step.tool_name, &step.arguments) {
                        Ok(()) => {
                            tier1_executed += 1;
                            step_results.push(DryRunStepResult {
                                step_index: step.step_index,
                                tool_name: step.tool_name.clone(),
                                risk_tier: 1,
                                simulated: false,
                                success: true,
                                message: "Tier 1 inspection tool executed and validated live state"
                                    .to_string(),
                            });
                        }
                        Err(e) => {
                            all_passed = false;
                            step_results.push(DryRunStepResult {
                                step_index: step.step_index,
                                tool_name: step.tool_name.clone(),
                                risk_tier: 1,
                                simulated: false,
                                success: false,
                                message: format!("Tier 1 inspection failed: {}", e.user_message),
                            });
                        }
                    }
                }
                2 => {
                    // Tier 2: Mutating operation - STRICTLY SIMULATED, NEVER CALLED
                    tier2_simulated += 1;
                    step_results.push(DryRunStepResult {
                        step_index: step.step_index,
                        tool_name: step.tool_name.clone(),
                        risk_tier: 2,
                        simulated: true,
                        success: true,
                        message: "Tier 2 mutating step simulated offline; tool call suppressed for dry run safety".to_string(),
                    });
                }
                3 => {
                    // Tier 3: Destructive operation - STRICTLY SIMULATED, NEVER CALLED
                    tier3_simulated += 1;
                    step_results.push(DryRunStepResult {
                        step_index: step.step_index,
                        tool_name: step.tool_name.clone(),
                        risk_tier: 3,
                        simulated: true,
                        success: true,
                        message: "Tier 3 destructive step simulated offline; tool call suppressed for dry run safety".to_string(),
                    });
                }
                _ => {
                    all_passed = false;
                    step_results.push(DryRunStepResult {
                        step_index: step.step_index,
                        tool_name: step.tool_name.clone(),
                        risk_tier: step.risk_tier,
                        simulated: true,
                        success: false,
                        message: format!("Invalid risk tier: {}", step.risk_tier),
                    });
                }
            }
        }

        Ok(DryRunReport {
            total_steps: plan.steps.len(),
            tier1_executed,
            tier2_simulated,
            tier3_simulated,
            step_results,
            all_passed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::tasksheet::TaskSheetStep;

    #[test]
    fn test_dry_run_never_calls_tier_2_or_3_tools() {
        // Verification requirement for 12.5:
        // "Dry run executes Tier 1 inspection steps only and validates arguments against live app state.
        // Verify: dry run never calls a Tier 2/3 tool (test asserts)."

        let plan = TaskSheet::new(
            "Complete media workflow demo",
            "Media Assembly",
            "Full pipeline with inspection, mutation, and cleanup",
            vec![
                TaskSheetStep {
                    step_index: 0,
                    app_id: "ssp".to_string(),
                    tool_name: "ssp.recording.start".to_string(),
                    arguments: serde_json::json!({ "fps": 60 }),
                    risk_tier: 1, // Tier 1: Inspection / safe
                    diff_summary: "Start recording".to_string(),
                    depends_on: vec![],
                },
                TaskSheetStep {
                    step_index: 1,
                    app_id: "eep".to_string(),
                    tool_name: "eep.timeline.cut_interval".to_string(),
                    arguments: serde_json::json!({ "startMs": 0, "endMs": 1000 }),
                    risk_tier: 2, // Tier 2: Mutating
                    diff_summary: "Cut interval".to_string(),
                    depends_on: [0].to_vec(),
                },
                TaskSheetStep {
                    step_index: 2,
                    app_id: "atp".to_string(),
                    tool_name: "atp.cache.clean".to_string(),
                    arguments: serde_json::json!({ "target": "package", "dryRun": true }),
                    risk_tier: 3, // Tier 3: Destructive
                    diff_summary: "Wipe package cache".to_string(),
                    depends_on: [1].to_vec(),
                },
            ],
        );

        let preview = PlanPreview::from_tasksheet(&plan);
        assert_eq!(preview.total_steps, 3);
        assert!(preview.requires_human_approval);
        assert_eq!(preview.tier_counts.get(&1), Some(&1));
        assert_eq!(preview.tier_counts.get(&2), Some(&1));
        assert_eq!(preview.tier_counts.get(&3), Some(&1));
        assert_eq!(preview.required_checkpoints.len(), 2);

        let mut tier1_calls = 0;
        let mut tier2_calls = 0;
        let mut tier3_calls = 0;

        let mut mock_caller = |_app: &str, tool: &str, _args: &serde_json::Value| {
            if tool == "ssp.recording.start" {
                tier1_calls += 1;
            } else if tool == "eep.timeline.cut_interval" {
                tier2_calls += 1;
            } else if tool == "atp.cache.clean" {
                tier3_calls += 1;
            }
            Ok(())
        };

        let validator = TaskSheetValidator::default();
        let report = DryRunExecutor::execute(&plan, &validator, &mut mock_caller).unwrap();

        assert!(report.all_passed);
        assert_eq!(report.total_steps, 3);
        assert_eq!(report.tier1_executed, 1);
        assert_eq!(report.tier2_simulated, 1);
        assert_eq!(report.tier3_simulated, 1);

        // HARD SAFETY ASSERTIONS:
        assert_eq!(
            tier1_calls, 1,
            "Expected Tier 1 tool to be called during dry run"
        );
        assert_eq!(
            tier2_calls, 0,
            "VIOLATION: Tier 2 tool was called during dry run!"
        );
        assert_eq!(
            tier3_calls, 0,
            "VIOLATION: Tier 3 tool was called during dry run!"
        );
    }
}
