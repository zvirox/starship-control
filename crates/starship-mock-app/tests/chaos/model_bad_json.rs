use super::harness::ChaosMission;
use starship_core::CoreError;
use starship_core::MissionState;
use starship_core::executor::{FailureClass, RetryPolicy, classify_error};
use starship_core::planner::{
    PlanGenerator, PlannerRepairLoop, RepairResult, TaskSheet, TaskSheetStep, TaskSheetValidator,
};

struct ScriptedModel {
    outputs: Vec<String>,
}

impl PlanGenerator for ScriptedModel {
    fn generate_plan(&mut self, _prompt: &str) -> Result<String, CoreError> {
        Ok(self.outputs.remove(0))
    }
}

fn valid_tasksheet_json() -> String {
    TaskSheet::new(
        "record screen",
        "Recording",
        "Start a screen recording",
        vec![TaskSheetStep {
            step_index: 0,
            app_id: "ssp".to_owned(),
            tool_name: "ssp.recording.start".to_owned(),
            arguments: serde_json::json!({ "fps": 60, "displayId": 0 }),
            risk_tier: 1,
            diff_summary: "Starts recording".to_owned(),
            depends_on: Vec::new(),
        }],
    )
    .to_json()
    .expect("serialize tasksheet")
}

#[test]
fn model_bad_json_is_a_non_retryable_plan_failure() {
    let chaos = ChaosMission::new();
    chaos.start_step();

    let error = TaskSheet::from_llm_json("{ not json").expect_err("malformed model output");
    assert_eq!(error.code, "planner.deserialization_failed");
    // Current classifier gap: planner.deserialization_failed falls through to AppFailure.
    assert_eq!(classify_error(&error.code), FailureClass::AppFailure);
    let retry = RetryPolicy::default().evaluate_retry(
        starship_core::safety::RiskTier::Safe,
        FailureClass::ModelOutput,
        true,
        true,
        1,
    );
    assert!(
        !retry.should_retry,
        "bad model output must not be retried blindly"
    );

    chaos.fail_step(&error.code);
    chaos.assert_final(MissionState::Failed, Some(&error.code));
}

#[test]
fn planner_repair_loop_recovers_after_bad_json() {
    let chaos = ChaosMission::new();
    let mut model = ScriptedModel {
        outputs: vec!["{ truncated".to_owned(), valid_tasksheet_json()],
    };
    let loop_engine = PlannerRepairLoop::new(TaskSheetValidator::default());

    let result = loop_engine
        .run("initial prompt", "record screen", &mut model)
        .expect("repair loop result");
    match result {
        RepairResult::Accepted {
            attempts_used,
            plan,
        } => {
            assert_eq!(attempts_used, 2);
            assert_eq!(plan.steps.len(), 1);
        }
        RepairResult::FallbackEscalated { .. } => panic!("repair should have recovered"),
    }
    chaos.audit("chaos.planner_repair", "success", None);
    chaos.assert_final(MissionState::Running, None);
}
