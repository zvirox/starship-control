use crate::error::CoreError;
use crate::executor::fallback::FallbackRung;
use crate::planner::tasksheet::TaskSheet;
use crate::planner::validator::TaskSheetValidator;
use std::fmt::Write as _;

/// Default number of repair attempts on validation failure (GAP-CLOSURE-PLAN.md §12.2).
pub const DEFAULT_MAX_REPAIR_ATTEMPTS: usize = 2;

/// Outcome of running the planner repair loop.
#[derive(Debug, Clone, PartialEq)]
pub enum RepairResult {
    /// Plan successfully validated (immediately or after repair iterations).
    Accepted {
        plan: TaskSheet,
        attempts_used: usize,
    },
    /// Repair iterations exhausted (≤ 2 attempts); escalated to F3 Fallback ladder rung.
    FallbackEscalated {
        attempts_exhausted: usize,
        last_error: CoreError,
        fallback_rung: FallbackRung,
    },
}

/// Abstract generator trait for producing or re-generating LLM plans.
pub trait PlanGenerator {
    fn generate_plan(&mut self, prompt: &str) -> Result<String, CoreError>;
}

impl<F> PlanGenerator for F
where
    F: FnMut(&str) -> Result<String, CoreError>,
{
    fn generate_plan(&mut self, prompt: &str) -> Result<String, CoreError> {
        self(prompt)
    }
}

/// Self-correcting planner repair loop (GAP-CLOSURE-PLAN.md §12.2).
#[derive(Debug, Clone)]
pub struct PlannerRepairLoop {
    validator: TaskSheetValidator,
    max_repair_attempts: usize,
}

impl Default for PlannerRepairLoop {
    fn default() -> Self {
        Self::new(TaskSheetValidator::default())
    }
}

impl PlannerRepairLoop {
    /// Creates a new repair loop with default limits (≤ 2 repair attempts).
    #[must_use]
    pub fn new(validator: TaskSheetValidator) -> Self {
        Self {
            validator,
            max_repair_attempts: DEFAULT_MAX_REPAIR_ATTEMPTS,
        }
    }

    /// Customizes the maximum allowed repair attempts before escalating to F3.
    #[must_use]
    pub fn with_max_repair_attempts(mut self, attempts: usize) -> Self {
        self.max_repair_attempts = attempts;
        self
    }

    /// Constructs a self-correction repair prompt with untrusted context fences.
    #[must_use]
    pub fn build_repair_prompt(user_goal: &str, invalid_output: &str, error: &CoreError) -> String {
        let mut prompt = String::new();
        prompt.push_str(
            "Your previous generated tasksheet failed validation and cannot be executed.\n\n\
             ### Specific Validation Error:\n",
        );
        let _ = writeln!(prompt, "- **Error Code:** `{}`", error.code);
        let _ = writeln!(prompt, "- **Reason:** {}\n", error.user_message);

        prompt.push_str("### Previous Invalid Output:\n<untrusted_context>\n");
        prompt.push_str(invalid_output);
        prompt.push_str("\n</untrusted_context>\n\n");

        prompt.push_str("### User Strategic Goal:\n<untrusted_context>\n");
        prompt.push_str(user_goal);
        prompt.push_str(
            "\n</untrusted_context>\n\n\
             Fix the validation error and return a fully corrected, valid JSON tasksheet adhering strictly to tasksheet/1 schema.\n\
             Output ONLY valid JSON:\n",
        );

        prompt
    }

    /// Executes the repair loop: attempts initial generation, validates, and loops up to `max_repair_attempts` on failure.
    pub fn run<G: PlanGenerator>(
        &self,
        initial_prompt: &str,
        user_goal: &str,
        generator: &mut G,
    ) -> Result<RepairResult, CoreError> {
        let mut current_prompt = initial_prompt.to_string();
        let mut attempts = 0;
        let mut last_error: Option<CoreError> = None;

        while attempts <= self.max_repair_attempts {
            attempts += 1;
            let raw_output = generator.generate_plan(&current_prompt)?;

            match TaskSheet::from_llm_json(&raw_output) {
                Ok(sheet) => match self.validator.validate(&sheet) {
                    Ok(()) => {
                        return Ok(RepairResult::Accepted {
                            plan: sheet,
                            attempts_used: attempts,
                        });
                    }
                    Err(val_err) => {
                        current_prompt =
                            Self::build_repair_prompt(user_goal, &raw_output, &val_err);
                        last_error = Some(val_err);
                    }
                },
                Err(parse_err) => {
                    current_prompt = Self::build_repair_prompt(user_goal, &raw_output, &parse_err);
                    last_error = Some(parse_err);
                }
            }
        }

        // All repair attempts exhausted -> Escalate to F3 Fallback chain (§2.2 F3)
        let final_error = last_error.unwrap_or_else(|| {
            CoreError::plan(
                "plan.replan_exhausted",
                false,
                "Plan validation repairs exhausted",
                None,
            )
        });

        Ok(RepairResult::FallbackEscalated {
            attempts_exhausted: attempts,
            last_error: final_error,
            fallback_rung: FallbackRung::F3Replan,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockEngine {
        responses: Vec<String>,
        calls: usize,
    }

    impl PlanGenerator for MockEngine {
        fn generate_plan(&mut self, _prompt: &str) -> Result<String, CoreError> {
            let res = self
                .responses
                .get(self.calls)
                .cloned()
                .unwrap_or_else(|| self.responses.last().unwrap().clone());
            self.calls += 1;
            Ok(res)
        }
    }

    const BAD_JSON_1: &str = r#"{
        "version": 1,
        "goal": "Record screen",
        "title": "Bad 1",
        "summary": "Duplicate index",
        "steps": [
            {
                "step_index": 0,
                "app_id": "ssp",
                "tool_name": "ssp.recording.start",
                "arguments": { "fps": 60 },
                "risk_tier": 1,
                "diff_summary": "start",
                "depends_on": []
            },
            {
                "step_index": 0,
                "app_id": "ssp",
                "tool_name": "ssp.recording.stop",
                "arguments": {},
                "risk_tier": 1,
                "diff_summary": "stop",
                "depends_on": []
            }
        ]
    }"#;

    const BAD_JSON_2: &str = r#"{
        "version": 1,
        "goal": "Record screen",
        "title": "Bad 2",
        "summary": "Unknown tool",
        "steps": [
            {
                "step_index": 0,
                "app_id": "ssp",
                "tool_name": "ssp.unregistered_alien_tool",
                "arguments": {},
                "risk_tier": 1,
                "diff_summary": "alien",
                "depends_on": []
            }
        ]
    }"#;

    const GOOD_JSON: &str = r#"{
        "version": 1,
        "goal": "Record screen",
        "title": "Screen Recording",
        "summary": "Records the display output cleanly",
        "steps": [
            {
                "step_index": 0,
                "app_id": "ssp",
                "tool_name": "ssp.recording.start",
                "arguments": { "fps": 60 },
                "risk_tier": 1,
                "diff_summary": "start recording",
                "depends_on": []
            },
            {
                "step_index": 1,
                "app_id": "ssp",
                "tool_name": "ssp.recording.stop",
                "arguments": {},
                "risk_tier": 1,
                "diff_summary": "stop recording",
                "depends_on": [0]
            }
        ]
    }"#;

    #[test]
    fn test_repair_loop_bad_twice_then_good_accepts_plan() {
        // Verification requirement for 12.2:
        // Mock engine returns bad JSON twice then good -> plan accepted on 3rd attempt
        let mut mock = MockEngine {
            responses: vec![
                BAD_JSON_1.to_string(),
                BAD_JSON_2.to_string(),
                GOOD_JSON.to_string(),
            ],
            calls: 0,
        };

        let repair_loop = PlannerRepairLoop::default();
        let result = repair_loop
            .run("initial prompt", "Record screen", &mut mock)
            .unwrap();

        match result {
            RepairResult::Accepted {
                plan,
                attempts_used,
            } => {
                assert_eq!(attempts_used, 3);
                assert_eq!(plan.steps.len(), 2);
                assert_eq!(mock.calls, 3);
            }
            RepairResult::FallbackEscalated { .. } => {
                panic!("Expected Accepted, got FallbackEscalated");
            }
        }
    }

    #[test]
    fn test_repair_loop_third_failure_escalates_to_f3_fallback() {
        // Verification requirement for 12.2:
        // Mock engine returns bad JSON three times -> third failure escalates to F3 fallback
        let mut mock = MockEngine {
            responses: vec![
                BAD_JSON_1.to_string(),
                BAD_JSON_2.to_string(),
                BAD_JSON_1.to_string(),
            ],
            calls: 0,
        };

        let repair_loop = PlannerRepairLoop::default();
        let result = repair_loop
            .run("initial prompt", "Record screen", &mut mock)
            .unwrap();

        match result {
            RepairResult::FallbackEscalated {
                attempts_exhausted,
                fallback_rung,
                ..
            } => {
                assert_eq!(attempts_exhausted, 3);
                assert_eq!(fallback_rung, FallbackRung::F3Replan);
                assert_eq!(mock.calls, 3);
            }
            RepairResult::Accepted { .. } => {
                panic!("Expected FallbackEscalated, got Accepted");
            }
        }
    }
}
