use crate::CoreError;
use crate::pipeline::cet::{CetTelemetry, CutInterval};
use crate::planner::tasksheet::{TaskSheet, TaskSheetStep};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Parameters for assembling an end-to-end screencast pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreencastAssemblyRequest {
    pub recording_path: String,
    pub cet_path: Option<String>,
    pub clean_audio: bool,
    pub audio_preset: Option<String>,
    pub intro_script: Option<String>,
    pub target_timeline_track: u32,
}

/// Pipeline planning output containing calculated cuts and generated task sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreencastPipelineResult {
    pub task_sheet: TaskSheet,
    pub estimated_cuts_count: usize,
    pub total_trimmed_ms: u64,
    pub diff_summary: String,
}

/// Multi-app screencast pipeline orchestrator spanning SSP, ALP, and EEP.
pub struct ScreencastPipeline;

impl ScreencastPipeline {
    /// Builds a versioned `tasksheet/1` `TaskSheet` orchestrating SSP, ALP, and EEP.
    pub fn plan_assembly(
        request: &ScreencastAssemblyRequest,
        telemetry: Option<&CetTelemetry>,
    ) -> Result<ScreencastPipelineResult, CoreError> {
        let cuts: Vec<CutInterval> = if let Some(t) = telemetry {
            t.detect_cuts(1200) // Cut silences longer than 1.2s
        } else {
            // Default simulated cut points
            vec![
                CutInterval {
                    start_ms: 4500,
                    end_ms: 7200,
                    duration_ms: 2700,
                    reason: "Initial pause before demo intro".to_string(),
                },
                CutInterval {
                    start_ms: 22000,
                    end_ms: 28500,
                    duration_ms: 6500,
                    reason: "Dead air during application loading".to_string(),
                },
            ]
        };

        let total_trimmed_ms: u64 = cuts.iter().map(|c| c.duration_ms).sum();
        let total_secs = if total_trimmed_ms > 0 {
            (total_trimmed_ms + 500) / 1000
        } else {
            0
        };
        let mins = total_secs / 60;
        let rem_secs = total_secs % 60;
        let diff_summary = format!(
            "Applies {} ripple cuts and removes {mins:02}:{rem_secs:02} dead time via CET telemetry",
            cuts.len()
        );

        let mut steps = Vec::new();

        // Step 0: Inspect SSP Recording take and extract tracks
        steps.push(TaskSheetStep {
            step_index: 0,
            app_id: "ssp".to_string(),
            tool_name: "recording.inspect_take".to_string(),
            arguments: json!({
                "recordingPath": request.recording_path,
                "cetPath": request.cet_path,
            }),
            risk_tier: 1,
            diff_summary: format!(
                "Inspects video frames and audio channels in {}",
                request.recording_path
            ),
            depends_on: vec![],
        });

        // Step 1: ALP Audio Enhancement & Noise Removal
        let preset = request.audio_preset.as_deref().unwrap_or("voice_isolate");
        steps.push(TaskSheetStep {
            step_index: 1,
            app_id: "alp".to_string(),
            tool_name: "audio.cleanup".to_string(),
            arguments: json!({
                "sourceVideo": request.recording_path,
                "preset": preset,
                "dehum": true,
                "targetLufs": -14.0,
            }),
            risk_tier: 2,
            diff_summary: format!(
                "Isolates vocal take and removes background hum via ALP (preset: {preset})"
            ),
            depends_on: vec![0],
        });

        // Step 2 (Optional): ALP TTS Narration Synthesis
        let has_intro = request.intro_script.is_some();
        if let Some(ref script) = request.intro_script {
            steps.push(TaskSheetStep {
                step_index: 2,
                app_id: "alp".to_string(),
                tool_name: "tts.synthesize".to_string(),
                arguments: json!({
                    "text": script,
                    "voice": "natural_narrator",
                    "outputFormat": "wav",
                }),
                risk_tier: 2,
                diff_summary: format!(
                    "Synthesizes narration voiceover via ALP TTS ({} chars)",
                    script.len()
                ),
                depends_on: vec![0],
            });
        }

        // Step 3: EEP Timeline Ripple Cuts (from CET silences)
        let cuts_json: Vec<_> = cuts
            .iter()
            .map(|c| {
                json!({
                    "startMs": c.start_ms,
                    "endMs": c.end_ms,
                    "reason": c.reason,
                })
            })
            .collect();

        let cut_step_idx = if has_intro { 3 } else { 2 };
        steps.push(TaskSheetStep {
            step_index: cut_step_idx,
            app_id: "eep".to_string(),
            tool_name: "timeline.apply_cuts".to_string(),
            arguments: json!({
                "trackId": request.target_timeline_track,
                "rippleCut": true,
                "checkpointName": "#bk-04",
                "intervals": cuts_json,
            }),
            risk_tier: 2,
            diff_summary: diff_summary.clone(),
            depends_on: vec![1],
        });

        // Step 4: EEP Audio Track Alignment & Master Import
        let align_step_idx = cut_step_idx + 1;
        let mut align_deps = vec![1, cut_step_idx];
        if has_intro {
            align_deps.push(2);
        }

        steps.push(TaskSheetStep {
            step_index: align_step_idx,
            app_id: "eep".to_string(),
            tool_name: "timeline.align_audio".to_string(),
            arguments: json!({
                "videoTrack": request.target_timeline_track,
                "masteredAudio": "take_enhanced.wav",
                "introVoiceover": if has_intro { Some("intro_narration.wav") } else { None },
            }),
            risk_tier: 2,
            diff_summary: "Aligns mastered vocal take and intro voiceover on timeline".to_string(),
            depends_on: align_deps,
        });

        let task_sheet = TaskSheet::new(
            "Assemble and enhance screencast tutorial",
            "Screencast Assembly & Mastered Timeline Edit",
            format!(
                "Processes recording via CET telemetry, applies audio mastering in ALP, and ripple cuts timeline in EEP ({diff_summary})"
            ),
            steps,
        );

        task_sheet.validate()?;

        Ok(ScreencastPipelineResult {
            task_sheet,
            estimated_cuts_count: cuts.len(),
            total_trimmed_ms,
            diff_summary,
        })
    }
}
