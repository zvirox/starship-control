use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::io::BufRead;

/// Capture Event Track (CET) file header.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CetHeader {
    pub version: String,
    pub recording_fps: f64,
    pub duration_ms: u64,
    #[serde(default)]
    pub audio_sample_rate: u32,
}

impl Default for CetHeader {
    fn default() -> Self {
        Self {
            version: "cet/1.0".to_string(),
            recording_fps: 60.0,
            duration_ms: 0,
            audio_sample_rate: 48_000,
        }
    }
}

/// Discrete telemetry event recorded during a screencast take.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CetEvent {
    Header(CetHeader),
    Click {
        timestamp_ms: u64,
        button: String,
        x: f64,
        y: f64,
        window_title: Option<String>,
    },
    WindowSwitch {
        timestamp_ms: u64,
        app_name: String,
        window_title: String,
    },
    Silence {
        start_ms: u64,
        end_ms: u64,
        duration_ms: u64,
    },
    Marker {
        timestamp_ms: u64,
        label: String,
    },
    VoiceActivity {
        start_ms: u64,
        end_ms: u64,
        confidence: f64,
    },
}

/// Recommended cut segment calculated from CET telemetry analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CutInterval {
    pub start_ms: u64,
    pub end_ms: u64,
    pub duration_ms: u64,
    pub reason: String,
}

/// Parsed CET telemetry dataset for a screencast take.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CetTelemetry {
    pub header: CetHeader,
    pub events: Vec<CetEvent>,
}

impl CetTelemetry {
    /// Parses a multi-line `.cet.jsonl` string.
    pub fn from_jsonl_str(content: &str) -> Result<Self, CoreError> {
        let mut header = CetHeader::default();
        let mut events = Vec::new();

        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let event: CetEvent = serde_json::from_str(trimmed).map_err(|e| {
                CoreError::app(
                    "cet.parse_error",
                    false,
                    format!("Failed to parse CET line {}: {e}", idx + 1),
                    Some(serde_json::json!({ "line": trimmed })),
                )
            })?;

            if let CetEvent::Header(ref h) = event {
                header = h.clone();
            }
            events.push(event);
        }

        Ok(Self { header, events })
    }

    /// Reads and parses a `.cet.jsonl` from any buffered reader.
    pub fn from_reader<R: BufRead>(reader: R) -> Result<Self, CoreError> {
        let mut header = CetHeader::default();
        let mut events = Vec::new();

        for (idx, line_res) in reader.lines().enumerate() {
            let line = line_res.map_err(|e| {
                CoreError::app(
                    "cet.io_error",
                    false,
                    format!("Failed to read CET line {}: {e}", idx + 1),
                    None,
                )
            })?;

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let event: CetEvent = serde_json::from_str(trimmed).map_err(|e| {
                CoreError::app(
                    "cet.parse_error",
                    false,
                    format!("Failed to parse CET event on line {}: {e}", idx + 1),
                    Some(serde_json::json!({ "line": trimmed })),
                )
            })?;

            if let CetEvent::Header(ref h) = event {
                header = h.clone();
            }
            events.push(event);
        }

        Ok(Self { header, events })
    }

    /// Scans the telemetry for dead air / silences exceeding `min_silence_ms`
    /// and generates automated timeline cut markers.
    #[must_use]
    pub fn detect_cuts(&self, min_silence_ms: u64) -> Vec<CutInterval> {
        let mut cuts = Vec::new();

        for event in &self.events {
            if let CetEvent::Silence {
                start_ms,
                end_ms,
                duration_ms,
            } = event
                && *duration_ms >= min_silence_ms
            {
                let secs = duration_ms / 1000;
                let tenths = (duration_ms % 1000) / 100;
                let formatted_duration = format!("{secs}.{tenths}s");
                cuts.push(CutInterval {
                    start_ms: *start_ms,
                    end_ms: *end_ms,
                    duration_ms: *duration_ms,
                    reason: format!("Silence interval of {formatted_duration}"),
                });
            }
        }

        cuts
    }

    /// Calculates total trimmed duration in milliseconds.
    #[must_use]
    pub fn total_trimmed_ms(&self, cuts: &[CutInterval]) -> u64 {
        cuts.iter().map(|c| c.duration_ms).sum()
    }

    /// Generates human-readable diff summary for approval cards.
    #[must_use]
    pub fn format_diff_summary(&self, cuts: &[CutInterval]) -> String {
        let total_ms = self.total_trimmed_ms(cuts);
        let total_secs = if total_ms > 0 {
            (total_ms + 500) / 1000
        } else {
            0
        };
        let mins = total_secs / 60;
        let rem_secs = total_secs % 60;
        let time_str = format!("{mins:02}:{rem_secs:02}");

        format!(
            "Applies {} ripple cuts and removes {time_str} dead time via CET telemetry",
            cuts.len()
        )
    }
}
