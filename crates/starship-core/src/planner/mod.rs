pub mod dag;
pub mod prompt;
pub mod tasksheet;

pub use dag::PlanDag;
pub use prompt::PlannerPromptBuilder;
pub use tasksheet::{TASKSHEET_VERSION_1, TaskSheet, TaskSheetStep};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_record_screen_and_clean_audio_decomposition() {
        // Verification requirement for 5.1:
        // "Record screen and clean audio" decomposes into distinct sequential steps for SSP and ALP.
        let steps = vec![
            TaskSheetStep {
                step_index: 0,
                app_id: "ssp".to_string(),
                tool_name: "recording.capture_screen".to_string(),
                arguments: json!({ "duration_secs": 60, "include_mic": true }),
                risk_tier: 1,
                diff_summary: "Captures 60 seconds screen recording with microphone audio"
                    .to_string(),
                depends_on: vec![],
            },
            TaskSheetStep {
                step_index: 1,
                app_id: "alp".to_string(),
                tool_name: "audio.extract_and_cleanup".to_string(),
                arguments: json!({ "preset": "voice_isolate", "dehum": true }),
                risk_tier: 2,
                diff_summary: "Extracts vocal track and cleans noise in Audio Labs Pro".to_string(),
                depends_on: vec![0],
            },
            TaskSheetStep {
                step_index: 2,
                app_id: "eep".to_string(),
                tool_name: "timeline.import_take".to_string(),
                arguments: json!({ "track": 1, "align_cet": true }),
                risk_tier: 2,
                diff_summary: "Imports synchronized video and mastered audio to timeline"
                    .to_string(),
                depends_on: vec![1],
            },
        ];

        let sheet = TaskSheet::new(
            "Record screen and clean audio",
            "Screen Recording & Mastered Audio Assembly",
            "Records desktop demo, cleans voice track via ALP, and inserts into EEP timeline",
            steps,
        );

        assert!(sheet.validate().is_ok());
        assert!(sheet.requires_approval());

        let dag = PlanDag::build(&sheet).expect("DAG build failed");
        assert_eq!(dag.len(), 3);

        // Stages should be strictly sequential: stage 0 (step 0), stage 1 (step 1), stage 2 (step 2)
        let stages = dag.stages();
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0], vec![0]);
        assert_eq!(stages[1], vec![1]);
        assert_eq!(stages[2], vec![2]);
    }

    #[test]
    fn test_parallel_branches_dag() {
        let steps = vec![
            TaskSheetStep {
                step_index: 0,
                app_id: "nwp".to_string(),
                tool_name: "network.scan".to_string(),
                arguments: json!({}),
                risk_tier: 1,
                diff_summary: "Queries network telemetry".to_string(),
                depends_on: vec![],
            },
            TaskSheetStep {
                step_index: 1,
                app_id: "atp".to_string(),
                tool_name: "disk.scan".to_string(),
                arguments: json!({}),
                risk_tier: 1,
                diff_summary: "Scans disk usage".to_string(),
                depends_on: vec![],
            },
            TaskSheetStep {
                step_index: 2,
                app_id: "atp".to_string(),
                tool_name: "system.generate_report".to_string(),
                arguments: json!({}),
                risk_tier: 1,
                diff_summary: "Aggregates metrics".to_string(),
                depends_on: vec![0, 1],
            },
        ];

        let sheet = TaskSheet::new(
            "System Health Scan",
            "Health Check",
            "Scan disk and net",
            steps,
        );
        let dag = PlanDag::build(&sheet).expect("DAG build failed");

        let stages = dag.stages();
        assert_eq!(stages.len(), 2);
        // Stage 0 contains both 0 and 1 in parallel
        assert_eq!(stages[0], vec![0, 1]);
        // Stage 1 contains 2
        assert_eq!(stages[1], vec![2]);
    }

    #[test]
    fn test_cycle_detection_fails() {
        let steps = vec![
            TaskSheetStep {
                step_index: 0,
                app_id: "ssp".to_string(),
                tool_name: "step_0".to_string(),
                arguments: json!({}),
                risk_tier: 1,
                diff_summary: String::new(),
                depends_on: vec![1], // Depends on 1
            },
            TaskSheetStep {
                step_index: 1,
                app_id: "alp".to_string(),
                tool_name: "step_1".to_string(),
                arguments: json!({}),
                risk_tier: 1,
                diff_summary: String::new(),
                depends_on: vec![0], // Depends on 0 (cycle!)
            },
        ];

        let sheet = TaskSheet::new("Cyclic test", "Cycle", "", steps);
        let res = PlanDag::build(&sheet);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.code, "planner.cyclic_dependency");
    }

    #[test]
    fn test_parse_llm_json_fenced() {
        let raw = r#"
        Here is the planned task sheet for your goal:
        ```json
        {
          "version": 1,
          "goal": "Clean audio",
          "title": "Audio Cleanup",
          "summary": "Cleans background noise",
          "steps": [
            {
              "step_index": 0,
              "app_id": "alp",
              "tool_name": "audio.cleanup",
              "arguments": { "preset": "voice_clean" },
              "risk_tier": 2,
              "diff_summary": "Cleans voice noise",
              "depends_on": []
            }
          ]
        }
        ```
        Let me know if you would like me to proceed.
        "#;

        let sheet = PlannerPromptBuilder::parse_llm_plan(raw).expect("parse markdown json");
        assert_eq!(sheet.version, 1);
        assert_eq!(sheet.steps.len(), 1);
        assert_eq!(sheet.steps[0].tool_name, "audio.cleanup");
        assert_eq!(sheet.steps[0].risk_tier, 2);
    }
}
