use crate::CoreError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Format version identifier for `tasksheet/1`.
pub const TASKSHEET_VERSION_1: u32 = 1;

/// Canonical execution plan adhering to `tasksheet/1`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskSheet {
    /// Format version number (must be 1 for `tasksheet/1`).
    pub version: u32,
    /// High-level goal or mission prompt.
    pub goal: String,
    /// Title or short description of the workflow.
    #[serde(default)]
    pub title: String,
    /// Detailed summary of what the plan achieves.
    #[serde(default)]
    pub summary: String,
    /// Sequence of discrete action steps forming the DAG.
    pub steps: Vec<TaskSheetStep>,
}

/// Discrete unit of tool execution in a `tasksheet/1` plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskSheetStep {
    /// Zero-based or ordinal index of this step.
    pub step_index: u32,
    /// Target application canonical ID (`eep`, `ssp`, `alp`, `nwp`, `atp`).
    pub app_id: String,
    /// MCP tool identifier (e.g. `timeline.cut_interval`, `audio.cleanup`).
    pub tool_name: String,
    /// Serialized tool parameters.
    pub arguments: Value,
    /// Safety classification: 1 = Safe/Read-only, 2 = Mutating, 3 = Destructive.
    pub risk_tier: u8,
    /// Human-readable explanation of impact (e.g. "Cuts 18 silence intervals").
    pub diff_summary: String,
    /// Step indices that must complete before this step can execute.
    #[serde(default)]
    pub depends_on: Vec<u32>,
}

impl TaskSheet {
    /// Creates a new `TaskSheet` with format version 1.
    #[must_use]
    pub fn new(
        goal: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        steps: Vec<TaskSheetStep>,
    ) -> Self {
        Self {
            version: TASKSHEET_VERSION_1,
            goal: goal.into(),
            title: title.into(),
            summary: summary.into(),
            steps,
        }
    }

    /// Validates the structure, step indices, and dependency references.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.version != TASKSHEET_VERSION_1 {
            return Err(CoreError::safety(
                "planner.unsupported_version",
                false,
                format!(
                    "Unsupported tasksheet version: {} (expected {})",
                    self.version, TASKSHEET_VERSION_1
                ),
                None,
            ));
        }

        let mut seen_indices = std::collections::HashSet::new();
        for step in &self.steps {
            if !seen_indices.insert(step.step_index) {
                return Err(CoreError::safety(
                    "planner.duplicate_step_index",
                    false,
                    format!("Duplicate step_index: {}", step.step_index),
                    None,
                ));
            }

            if !(1..=3).contains(&step.risk_tier) {
                return Err(CoreError::safety(
                    "planner.invalid_risk_tier",
                    false,
                    format!(
                        "Invalid risk_tier {} for step index {} (must be 1, 2, or 3)",
                        step.risk_tier, step.step_index
                    ),
                    None,
                ));
            }

            for &dep in &step.depends_on {
                if dep == step.step_index {
                    return Err(CoreError::safety(
                        "planner.self_dependency",
                        false,
                        format!("Step {} cannot depend on itself", step.step_index),
                        None,
                    ));
                }
            }
        }

        for step in &self.steps {
            for &dep in &step.depends_on {
                if !seen_indices.contains(&dep) {
                    return Err(CoreError::safety(
                        "planner.missing_dependency",
                        false,
                        format!(
                            "Step {} depends on non-existent step index {}",
                            step.step_index, dep
                        ),
                        None,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Returns true if any step requires human approval (Risk Tier 2 or 3).
    #[must_use]
    pub fn requires_approval(&self) -> bool {
        self.steps.iter().any(|s| s.risk_tier >= 2)
    }

    /// Serializes this `TaskSheet` to canonical JSON.
    pub fn to_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| {
            CoreError::safety(
                "planner.serialization_failed",
                false,
                format!("Failed to serialize tasksheet: {e}"),
                None,
            )
        })
    }

    /// Deserializes a `TaskSheet` from JSON and validates it.
    pub fn from_json(json_str: &str) -> Result<Self, CoreError> {
        let sheet: Self = serde_json::from_str(json_str).map_err(|e| {
            CoreError::safety(
                "planner.deserialization_failed",
                false,
                format!("Invalid tasksheet JSON: {e}"),
                None,
            )
        })?;
        sheet.validate()?;
        Ok(sheet)
    }

    /// Deserializes a `TaskSheet` from untrusted LLM-generated JSON.
    /// Invariants enforced:
    /// 1. Any LLM-supplied `diff_summary` is strictly ignored and replaced with
    ///    host-deterministic summaries from `DiffSummarizer` (GAP-CLOSURE-PLAN.md §10.2).
    /// 2. Any LLM-declared `risk_tier` is elevated to host floor via `TierPolicy` (GAP-CLOSURE-PLAN.md §10.1).
    pub fn from_llm_json(json_str: &str) -> Result<Self, CoreError> {
        let mut sheet = Self::from_json(json_str)?;
        let policy = crate::policy::TierPolicy::default();

        for step in &mut sheet.steps {
            // Strictly overwrite with host-computed deterministic diff summary
            step.diff_summary = crate::policy::DiffSummarizer::summarize(
                &step.app_id,
                &step.tool_name,
                &step.arguments,
            );

            // Enforce host floor on risk tier
            let app_declared = crate::safety::RiskTier::from_u8(step.risk_tier);
            step.risk_tier = policy
                .calculate_effective_tier(app_declared, &step.tool_name)
                .as_u8();
        }

        Ok(sheet)
    }
}
