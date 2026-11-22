use crate::error::CoreError;
use crate::planner::tasksheet::{TASKSHEET_VERSION_1, TaskSheet, TaskSheetStep};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};

/// Default maximum number of steps allowed in a single plan (GAP-CLOSURE-PLAN.md §12.1).
pub const DEFAULT_MAX_STEPS: usize = 30;

/// Default maximum number of Tier 3 (destructive) steps allowed in a plan (GAP-CLOSURE-PLAN.md §12.1).
pub const DEFAULT_MAX_TIER3_STEPS: usize = 3;

/// Tool schema metadata used for argument and permission validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub app_id: String,
    pub tool_name: String,
    pub risk_tier: u8,
    pub parameters_schema: Value,
}

impl ToolSchema {
    #[must_use]
    pub fn new(
        app_id: impl Into<String>,
        tool_name: impl Into<String>,
        risk_tier: u8,
        parameters_schema: Value,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            tool_name: tool_name.into(),
            risk_tier,
            parameters_schema,
        }
    }
}

/// `TaskSheet` validator ensuring plans are valid by construction (GAP-CLOSURE-PLAN.md §12.1).
#[derive(Debug, Clone)]
pub struct TaskSheetValidator {
    pub max_steps: usize,
    pub max_tier3_steps: usize,
    known_apps: HashSet<String>,
    tools: HashMap<String, ToolSchema>,
}

impl Default for TaskSheetValidator {
    fn default() -> Self {
        let mut validator = Self {
            max_steps: DEFAULT_MAX_STEPS,
            max_tier3_steps: DEFAULT_MAX_TIER3_STEPS,
            known_apps: HashSet::new(),
            tools: HashMap::new(),
        };

        // Standard 5 Starship apps
        for app in ["eep", "ssp", "alp", "nwp", "atp"] {
            validator.register_app(app);
        }

        // Register default ecosystem tools with canonical parameter schemas
        validator.register_tool(ToolSchema::new(
            "eep",
            "eep.timeline.cut_interval",
            2,
            serde_json::json!({
                "type": "object",
                "properties": {
                    "startMs": { "type": "integer" },
                    "endMs": { "type": "integer" }
                },
                "required": ["startMs", "endMs"]
            }),
        ));

        validator.register_tool(ToolSchema::new(
            "eep",
            "eep.timeline.auto_trim_silence",
            2,
            serde_json::json!({
                "type": "object",
                "properties": {
                    "thresholdDb": { "type": "number" },
                    "minSilenceMs": { "type": "integer" }
                }
            }),
        ));

        validator.register_tool(ToolSchema::new(
            "ssp",
            "ssp.recording.start",
            1,
            serde_json::json!({
                "type": "object",
                "properties": {
                    "fps": { "type": "integer" },
                    "displayId": { "type": "integer" }
                }
            }),
        ));

        validator.register_tool(ToolSchema::new(
            "ssp",
            "ssp.recording.stop",
            1,
            serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        ));

        validator.register_tool(ToolSchema::new(
            "alp",
            "alp.denoise.spectral",
            2,
            serde_json::json!({
                "type": "object",
                "properties": {
                    "strength": { "type": "number" }
                }
            }),
        ));

        validator.register_tool(ToolSchema::new(
            "atp",
            "atp.cache.clean",
            3,
            serde_json::json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string" },
                    "dryRun": { "type": "boolean" }
                },
                "required": ["target"]
            }),
        ));

        validator
    }
}

impl TaskSheetValidator {
    /// Registers an application ID as known.
    pub fn register_app(&mut self, app_id: impl Into<String>) {
        self.known_apps.insert(app_id.into());
    }

    /// Registers a tool schema into the validator.
    pub fn register_tool(&mut self, tool: ToolSchema) {
        self.known_apps.insert(tool.app_id.clone());
        self.tools.insert(tool.tool_name.clone(), tool);
    }

    /// Validates a `TaskSheet` plan against all 8 schema and integrity gates.
    pub fn validate(&self, plan: &TaskSheet) -> Result<(), CoreError> {
        // Gate 1: Version check
        if plan.version != TASKSHEET_VERSION_1 {
            return Err(CoreError::plan(
                "planner.unsupported_version",
                false,
                format!(
                    "Unsupported tasksheet version: {} (expected {})",
                    plan.version, TASKSHEET_VERSION_1
                ),
                None,
            ));
        }

        // Gate 2: Goal presence
        if plan.goal.trim().is_empty() {
            return Err(CoreError::plan(
                "plan.empty_goal",
                false,
                "TaskSheet plan must contain a non-empty user goal",
                None,
            ));
        }

        // Gate 3: Steps count non-empty and max steps limit
        if plan.steps.is_empty() {
            return Err(CoreError::plan(
                "plan.empty_plan",
                false,
                "TaskSheet plan contains 0 steps; at least 1 step is required",
                None,
            ));
        }

        if plan.steps.len() > self.max_steps {
            return Err(CoreError::plan(
                "plan.max_steps_exceeded",
                false,
                format!(
                    "Plan step count {} exceeds maximum allowed limit of {}",
                    plan.steps.len(),
                    self.max_steps
                ),
                None,
            ));
        }

        // Gate 4: Max Tier 3 (destructive) steps limit
        let tier3_count = plan.steps.iter().filter(|s| s.risk_tier == 3).count();
        if tier3_count > self.max_tier3_steps {
            return Err(CoreError::plan(
                "plan.max_tier3_exceeded",
                false,
                format!(
                    "Plan contains {} Tier 3 destructive steps, exceeding limit of {}",
                    tier3_count, self.max_tier3_steps
                ),
                None,
            ));
        }

        // Gate 5: Step indices uniqueness and valid risk tiers
        let mut seen_indices = HashSet::with_capacity(plan.steps.len());
        for step in &plan.steps {
            if !seen_indices.insert(step.step_index) {
                return Err(CoreError::plan(
                    "plan.duplicate_step_index",
                    false,
                    format!("Duplicate step_index: {}", step.step_index),
                    None,
                ));
            }

            if !(1..=3).contains(&step.risk_tier) {
                return Err(CoreError::plan(
                    "plan.invalid_risk_tier",
                    false,
                    format!(
                        "Invalid risk_tier {} for step {} (must be 1, 2, or 3)",
                        step.risk_tier, step.step_index
                    ),
                    None,
                ));
            }

            // Gate 6: App and tool registration check
            if !self.known_apps.contains(&step.app_id) {
                return Err(CoreError::plan(
                    "plan.unknown_app",
                    false,
                    format!("Unknown target application ID: '{}'", step.app_id),
                    None,
                ));
            }

            let tool_schema = self.tools.get(&step.tool_name).ok_or_else(|| {
                CoreError::plan(
                    "plan.unknown_tool",
                    false,
                    format!(
                        "Tool '{}' does not exist in the active ecosystem registry",
                        step.tool_name
                    ),
                    None,
                )
            })?;

            if tool_schema.app_id != step.app_id {
                return Err(CoreError::plan(
                    "plan.unknown_tool",
                    false,
                    format!(
                        "Tool '{}' belongs to app '{}', not '{}'",
                        step.tool_name, tool_schema.app_id, step.app_id
                    ),
                    None,
                ));
            }

            // Gate 7: Arguments validation against tool parameter JSON schema
            Self::validate_arguments(step, &tool_schema.parameters_schema)?;
        }

        // Gate 8: Dependencies and DAG acyclicity check
        Self::validate_dependencies_and_dag(&plan.steps, &seen_indices)?;

        Ok(())
    }

    /// Validates arguments against parameter schema (required properties and primitive types).
    fn validate_arguments(step: &TaskSheetStep, schema: &Value) -> Result<(), CoreError> {
        let args_obj = step.arguments.as_object().ok_or_else(|| {
            CoreError::plan(
                "plan.args_invalid",
                false,
                format!("Step {} arguments must be a JSON object", step.step_index),
                None,
            )
        })?;

        // Check required fields
        if let Some(req_array) = schema.get("required").and_then(Value::as_array) {
            for req_val in req_array {
                if let Some(req_key) = req_val.as_str()
                    && (!args_obj.contains_key(req_key) || args_obj[req_key].is_null())
                {
                    return Err(CoreError::plan(
                        "plan.args_invalid",
                        false,
                        format!(
                            "Step {} tool '{}' missing required parameter '{}'",
                            step.step_index, step.tool_name, req_key
                        ),
                        None,
                    ));
                }
            }
        }

        // Check property types
        if let Some(props_obj) = schema.get("properties").and_then(Value::as_object) {
            for (prop_name, prop_schema) in props_obj {
                if let Some(arg_val) = args_obj.get(prop_name)
                    && let Some(expected_type) = prop_schema.get("type").and_then(Value::as_str)
                {
                    let type_valid = match expected_type {
                        "integer" => arg_val.is_i64() || arg_val.is_u64(),
                        "number" => arg_val.is_number(),
                        "string" => arg_val.is_string(),
                        "boolean" => arg_val.is_boolean(),
                        "array" => arg_val.is_array(),
                        "object" => arg_val.is_object(),
                        _ => true,
                    };

                    if !type_valid {
                        return Err(CoreError::plan(
                            "plan.args_invalid",
                            false,
                            format!(
                                "Step {} tool '{}' parameter '{}' expected {}, got {:?}",
                                step.step_index, step.tool_name, prop_name, expected_type, arg_val
                            ),
                            None,
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Validates dependency integrity and verifies the DAG is acyclic via Kahn's algorithm.
    fn validate_dependencies_and_dag(
        steps: &[TaskSheetStep],
        seen_indices: &HashSet<u32>,
    ) -> Result<(), CoreError> {
        let mut in_degrees: HashMap<u32, usize> = HashMap::new();
        let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();

        for step in steps {
            in_degrees.entry(step.step_index).or_insert(0);
            adj.entry(step.step_index).or_default();

            for &dep in &step.depends_on {
                if dep == step.step_index {
                    return Err(CoreError::plan(
                        "plan.self_dependency",
                        false,
                        format!("Step {} cannot depend on itself", step.step_index),
                        None,
                    ));
                }

                if !seen_indices.contains(&dep) {
                    return Err(CoreError::plan(
                        "plan.missing_dependency",
                        false,
                        format!(
                            "Step {} depends on non-existent step index {}",
                            step.step_index, dep
                        ),
                        None,
                    ));
                }

                // dep -> step.step_index
                adj.entry(dep).or_default().push(step.step_index);
                *in_degrees.entry(step.step_index).or_insert(0) += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<u32> = in_degrees
            .iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&idx, _)| idx)
            .collect();

        let mut visited_count = 0;
        while let Some(u) = queue.pop_front() {
            visited_count += 1;
            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    if let Some(deg) = in_degrees.get_mut(&v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v);
                        }
                    }
                }
            }
        }

        if visited_count != steps.len() {
            return Err(CoreError::plan(
                "plan.cycle_detected",
                false,
                "Dependency cycle detected in plan steps",
                None,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_step(index: u32) -> TaskSheetStep {
        TaskSheetStep {
            step_index: index,
            app_id: "ssp".to_string(),
            tool_name: "ssp.recording.start".to_string(),
            arguments: serde_json::json!({ "fps": 60 }),
            risk_tier: 1,
            diff_summary: "Start recording".to_string(),
            depends_on: vec![],
        }
    }

    #[test]
    fn test_valid_tasksheet_passes_validation() {
        let validator = TaskSheetValidator::default();
        let plan = TaskSheet::new(
            "Record 60 second screen demo",
            "Screen Demo",
            "Records display output",
            vec![valid_step(0), valid_step(1)],
        );
        assert!(validator.validate(&plan).is_ok());
    }

    // 15 Invalid Fixtures Testing Verification Matrix (GAP-CLOSURE-PLAN.md §12.1)

    #[test]
    fn test_fixture_01_unsupported_version() {
        let validator = TaskSheetValidator::default();
        let mut plan = TaskSheet::new("goal", "title", "summary", vec![valid_step(0)]);
        plan.version = 2; // Unsupported version
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "planner.unsupported_version");
    }

    #[test]
    fn test_fixture_02_empty_goal() {
        let validator = TaskSheetValidator::default();
        let plan = TaskSheet::new("   ", "title", "summary", vec![valid_step(0)]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.empty_goal");
    }

    #[test]
    fn test_fixture_03_empty_steps() {
        let validator = TaskSheetValidator::default();
        let plan = TaskSheet::new("goal", "title", "summary", vec![]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.empty_plan");
    }

    #[test]
    fn test_fixture_04_duplicate_step_index() {
        let validator = TaskSheetValidator::default();
        let plan = TaskSheet::new(
            "goal",
            "title",
            "summary",
            vec![valid_step(0), valid_step(0)],
        );
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.duplicate_step_index");
    }

    #[test]
    fn test_fixture_05_self_dependency() {
        let validator = TaskSheetValidator::default();
        let mut s0 = valid_step(0);
        s0.depends_on = vec![0];
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.self_dependency");
    }

    #[test]
    fn test_fixture_06_missing_dependency() {
        let validator = TaskSheetValidator::default();
        let mut s0 = valid_step(0);
        s0.depends_on = vec![99]; // Non-existent
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.missing_dependency");
    }

    #[test]
    fn test_fixture_07_direct_cycle() {
        let validator = TaskSheetValidator::default();
        let mut s0 = valid_step(0);
        let mut s1 = valid_step(1);
        s0.depends_on = vec![1];
        s1.depends_on = vec![0];
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0, s1]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.cycle_detected");
    }

    #[test]
    fn test_fixture_08_indirect_cycle() {
        let validator = TaskSheetValidator::default();
        let mut s0 = valid_step(0);
        let mut s1 = valid_step(1);
        let mut s2 = valid_step(2);
        s1.depends_on = vec![0];
        s2.depends_on = vec![1];
        s0.depends_on = vec![2];
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0, s1, s2]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.cycle_detected");
    }

    #[test]
    fn test_fixture_09_max_steps_exceeded() {
        let validator = TaskSheetValidator::default();
        let steps: Vec<TaskSheetStep> = (0..31).map(valid_step).collect();
        let plan = TaskSheet::new("goal", "title", "summary", steps);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.max_steps_exceeded");
    }

    #[test]
    fn test_fixture_10_max_tier3_exceeded() {
        let validator = TaskSheetValidator::default();
        let steps: Vec<TaskSheetStep> = (0..4)
            .map(|idx| TaskSheetStep {
                step_index: idx,
                app_id: "atp".to_string(),
                tool_name: "atp.cache.clean".to_string(),
                arguments: serde_json::json!({ "target": "all" }),
                risk_tier: 3,
                diff_summary: "Wipe cache".to_string(),
                depends_on: vec![],
            })
            .collect();
        let plan = TaskSheet::new("goal", "title", "summary", steps);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.max_tier3_exceeded");
    }

    #[test]
    fn test_fixture_11_invalid_risk_tier() {
        let validator = TaskSheetValidator::default();
        let mut s0 = valid_step(0);
        s0.risk_tier = 5; // Invalid tier
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.invalid_risk_tier");
    }

    #[test]
    fn test_fixture_12_unknown_app() {
        let validator = TaskSheetValidator::default();
        let mut s0 = valid_step(0);
        s0.app_id = "nonexistent_app".to_string();
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.unknown_app");
    }

    #[test]
    fn test_fixture_13_unknown_tool() {
        let validator = TaskSheetValidator::default();
        let mut s0 = valid_step(0);
        s0.tool_name = "ssp.unregistered_secret_tool".to_string();
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.unknown_tool");
    }

    #[test]
    fn test_fixture_14_args_type_mismatch() {
        let validator = TaskSheetValidator::default();
        let mut s0 = valid_step(0);
        s0.arguments = serde_json::json!({ "fps": "should_be_an_integer" });
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.args_invalid");
    }

    #[test]
    fn test_fixture_15_args_missing_required() {
        let validator = TaskSheetValidator::default();
        let s0 = TaskSheetStep {
            step_index: 0,
            app_id: "eep".to_string(),
            tool_name: "eep.timeline.cut_interval".to_string(),
            arguments: serde_json::json!({ "startMs": 100 }), // missing "endMs"
            risk_tier: 2,
            diff_summary: "Cut interval".to_string(),
            depends_on: vec![],
        };
        let plan = TaskSheet::new("goal", "title", "summary", vec![s0]);
        let err = validator.validate(&plan).unwrap_err();
        assert_eq!(err.code, "plan.args_invalid");
    }
}
