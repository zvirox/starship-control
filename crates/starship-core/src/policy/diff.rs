use serde_json::Value;

/// Host-side deterministic diff summarizer (GAP-CLOSURE-PLAN.md §10.2).
/// Invariant: `diffSummary` is ALWAYS computed deterministically from verified arguments
/// or tool metadata. LLM-provided diff summaries are strictly ignored to prevent
/// hallucination and deceptive prompt injection.
#[derive(Debug, Clone, Copy)]
pub struct DiffSummarizer;

impl DiffSummarizer {
    /// Generates a human-verifiable summary strictly from tool name and arguments.
    /// If no specific pattern matches, formats raw arguments so the approval card
    /// shows the exact parameters being executed.
    #[must_use]
    pub fn summarize(app_id: &str, tool_name: &str, args: &Value) -> String {
        let app_lower = app_id.to_ascii_lowercase();
        let tool_lower = tool_name.to_ascii_lowercase();

        match (app_lower.as_str(), tool_lower.as_str()) {
            // Easy Edits Pro: Timeline cuts
            ("eep", "timeline.cut" | "timeline.cut_interval") => {
                if let Some(cuts) = args.get("cuts").and_then(Value::as_array) {
                    format!("Applies {} cut intervals to video timeline", cuts.len())
                } else if let (Some(start), Some(end)) = (args.get("start_ms"), args.get("end_ms"))
                {
                    format!("Cuts timeline interval between {start}ms and {end}ms")
                } else {
                    format!("Modifies video timeline: {}", Self::compact_args(args))
                }
            }
            ("eep", "timeline.import_take") => {
                let track = args.get("track").and_then(Value::as_i64).unwrap_or(1);
                format!("Imports media take onto video track {track}")
            }
            // Audio Labs Pro: DSP & Voice
            ("alp", "audio.cleanup" | "audio.extract_and_cleanup") => {
                let preset = args
                    .get("preset")
                    .and_then(Value::as_str)
                    .unwrap_or("default");
                format!("Cleans audio noise using '{preset}' DSP preset")
            }
            ("alp", "voice.synthesize") => {
                let text_len = args.get("text").and_then(Value::as_str).map_or(0, str::len);
                format!("Synthesizes TTS speech narration ({text_len} chars)")
            }
            // Agentic Tuneup Pro: Package cache purges
            ("atp", "purge_package_cache" | "clean_cache") => {
                let manager = args
                    .get("manager")
                    .and_then(Value::as_str)
                    .unwrap_or("system");
                format!("Permanently purges {manager} package cache files")
            }
            // Generic file / resource deletions
            (_, name)
                if name.contains("delete") || name.contains("remove") || name.contains("purge") =>
            {
                if let Some(path) = args.get("path").and_then(Value::as_str) {
                    format!("Permanently deletes path: {path}")
                } else {
                    format!("Deletes resource: {}", Self::compact_args(args))
                }
            }
            // Default fallback: show exact raw arguments
            _ => {
                if args.is_null()
                    || (args.is_object() && args.as_object().is_some_and(serde_json::Map::is_empty))
                {
                    format!("Executes {tool_name}")
                } else {
                    // Show raw arguments for user inspection
                    serde_json::to_string(args).unwrap_or_else(|_| "{}".to_string())
                }
            }
        }
    }

    fn compact_args(args: &Value) -> String {
        if args.is_null() {
            return String::new();
        }
        serde_json::to_string(args).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_diff_summarizer_deterministic_formatting() {
        // eep cut
        let eep_summary = DiffSummarizer::summarize(
            "eep",
            "timeline.cut",
            &json!({ "cuts": [ {"start": 0, "end": 10}, {"start": 20, "end": 30} ] }),
        );
        assert_eq!(eep_summary, "Applies 2 cut intervals to video timeline");

        // atp purge
        let atp_summary = DiffSummarizer::summarize(
            "atp",
            "purge_package_cache",
            &json!({ "manager": "pacman" }),
        );
        assert_eq!(atp_summary, "Permanently purges pacman package cache files");

        // delete path
        let del_summary = DiffSummarizer::summarize(
            "fs",
            "files.delete",
            &json!({ "path": "/var/log/system.log" }),
        );
        assert_eq!(del_summary, "Permanently deletes path: /var/log/system.log");

        // unknown tool fallback to raw args
        let raw_summary = DiffSummarizer::summarize(
            "custom_app",
            "configure_hardware",
            &json!({ "voltage": 1.2, "clock_mhz": 3200 }),
        );
        assert_eq!(raw_summary, "{\"clock_mhz\":3200,\"voltage\":1.2}");
    }
}
