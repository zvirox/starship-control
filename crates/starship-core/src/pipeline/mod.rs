pub mod cet;
pub mod diagnostics;
pub mod media;

pub use cet::{CetEvent, CetHeader, CetTelemetry, CutInterval};
pub use diagnostics::{DiagnosticsPipeline, SystemMetricsReport};
pub use media::{ScreencastAssemblyRequest, ScreencastPipeline, ScreencastPipelineResult};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::PlanDag;

    #[test]
    fn test_cet_telemetry_parsing_and_cut_detection() {
        // Verification requirement for 7.1:
        // Read .cet.jsonl sidecar from SSP recording -> detect silences and clicks -> assemble edit markers -> dispatch cut commands to EEP.
        let jsonl_content = r#"
{"type":"header","version":"cet/1.0","recording_fps":60.0,"duration_ms":90000,"audio_sample_rate":48000}
{"type":"marker","timestamp_ms":0,"label":"Recording Started"}
{"type":"silence","start_ms":1000,"end_ms":4200,"duration_ms":3200}
{"type":"click","timestamp_ms":5500,"button":"left","x":410.5,"y":220.0,"window_title":"Easy Edits Pro"}
{"type":"silence","start_ms":15000,"end_ms":22500,"duration_ms":7500}
{"type":"window_switch","timestamp_ms":23000,"app_name":"ssp","window_title":"ScreenStudio Pro"}
{"type":"silence","start_ms":45000,"end_ms":45800,"duration_ms":800}
"#;

        let telemetry = CetTelemetry::from_jsonl_str(jsonl_content).expect("parse cet jsonl");
        assert_eq!(telemetry.header.version, "cet/1.0");
        assert_eq!(telemetry.events.len(), 7);

        // Detect silences longer than 1200ms
        let cuts = telemetry.detect_cuts(1200);
        assert_eq!(cuts.len(), 2);
        assert_eq!(cuts[0].start_ms, 1000);
        assert_eq!(cuts[0].end_ms, 4200);
        assert_eq!(cuts[0].duration_ms, 3200);

        assert_eq!(cuts[1].start_ms, 15000);
        assert_eq!(cuts[1].end_ms, 22500);
        assert_eq!(cuts[1].duration_ms, 7500);

        let total_ms = telemetry.total_trimmed_ms(&cuts);
        assert_eq!(total_ms, 10700);

        let summary = telemetry.format_diff_summary(&cuts);
        assert!(summary.contains("Applies 2 ripple cuts"));
        assert!(summary.contains("00:11 dead time"));
    }

    #[test]
    fn test_screencast_pipeline_dag_resolution() {
        // Verification requirement for 7.2:
        // Route extracted audio track to ALP for noise removal and voice mastering; synthesize intro narration via ALP TTS and insert into EEP timeline.
        let req = ScreencastAssemblyRequest {
            recording_path: "/home/user/Videos/demo-take-01.mkv".to_string(),
            cet_path: Some("/home/user/Videos/demo-take-01.cet.jsonl".to_string()),
            clean_audio: true,
            audio_preset: Some("voice_isolate".to_string()),
            intro_script: Some(
                "Welcome to this comprehensive Starship Control tutorial.".to_string(),
            ),
            target_timeline_track: 1,
        };

        let result = ScreencastPipeline::plan_assembly(&req, None).expect("plan screencast");
        assert_eq!(result.task_sheet.steps.len(), 5);

        // Check step ordering & app delegation
        assert_eq!(result.task_sheet.steps[0].app_id, "ssp");
        assert_eq!(
            result.task_sheet.steps[0].tool_name,
            "recording.inspect_take"
        );
        assert_eq!(result.task_sheet.steps[0].risk_tier, 1);

        assert_eq!(result.task_sheet.steps[1].app_id, "alp");
        assert_eq!(result.task_sheet.steps[1].tool_name, "audio.cleanup");
        assert_eq!(result.task_sheet.steps[1].risk_tier, 2);

        assert_eq!(result.task_sheet.steps[2].app_id, "alp");
        assert_eq!(result.task_sheet.steps[2].tool_name, "tts.synthesize");
        assert_eq!(result.task_sheet.steps[2].risk_tier, 2);

        assert_eq!(result.task_sheet.steps[3].app_id, "eep");
        assert_eq!(result.task_sheet.steps[3].tool_name, "timeline.apply_cuts");
        assert_eq!(result.task_sheet.steps[3].risk_tier, 2);

        assert_eq!(result.task_sheet.steps[4].app_id, "eep");
        assert_eq!(result.task_sheet.steps[4].tool_name, "timeline.align_audio");
        assert_eq!(result.task_sheet.steps[4].risk_tier, 2);

        // Validate DAG execution stages
        let dag = PlanDag::build(&result.task_sheet).expect("DAG build");
        assert_eq!(dag.len(), 5);
        let stages = dag.stages();
        assert_eq!(stages.len(), 4);
        assert_eq!(stages[0], vec![0]); // SSP inspection runs first
    }

    #[test]
    fn test_diagnostics_pipeline_generation() {
        // Verification requirement for 7.3:
        // Query network activity from NWP and disk storage metrics from ATP; generate cleanup task sheet upon approval.
        let report = SystemMetricsReport {
            live_connections_count: 14,
            total_bandwidth_kbps: 450.0,
            disk_used_gb: 120.5,
            disk_free_gb: 34.2,
            cache_bloat_mb: 4300.0, // 4.2 GB
            reclaimable_items: vec![
                "~/.cache/cargo/registry".to_string(),
                "~/.cache/npm".to_string(),
                "~/.cache/thumbnails".to_string(),
            ],
        };

        let sheet = DiagnosticsPipeline::plan_system_maintenance(&report);
        assert_eq!(sheet.steps.len(), 3);
        assert_eq!(sheet.steps[0].app_id, "nwp");
        assert_eq!(sheet.steps[0].risk_tier, 1);
        assert_eq!(sheet.steps[1].app_id, "atp");
        assert_eq!(sheet.steps[1].risk_tier, 1);

        // Final step is Tier 3 Destructive approval gate
        assert_eq!(sheet.steps[2].app_id, "atp");
        assert_eq!(sheet.steps[2].tool_name, "cache.purge_all");
        assert_eq!(sheet.steps[2].risk_tier, 3);
        assert!(sheet.requires_approval());

        let dag = PlanDag::build(&sheet).expect("DAG build");
        let stages = dag.stages();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0], vec![0, 1]); // NWP scan and ATP scan run in parallel
        assert_eq!(stages[1], vec![2]); // ATP cache purge runs after both scans complete
    }
}
