use crate::context::budget::TokenBudget;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// One-line application capability manifest used in Stage 1 tool routing (GAP-CLOSURE-PLAN.md §12.4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCapability {
    pub app_id: String,
    pub name: String,
    pub one_line_summary: String,
    pub keywords: Vec<String>,
}

/// Tool candidate for two-stage routing and BM25 ranking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteableTool {
    pub app_id: String,
    pub name: String,
    pub description: String,
    pub risk_tier: u8,
    pub parameters_summary: String,
}

impl RouteableTool {
    #[must_use]
    pub fn new(
        app_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        risk_tier: u8,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            name: name.into(),
            description: description.into(),
            risk_tier,
            parameters_summary: "{}".to_string(),
        }
    }
}

/// Two-stage tool router with BM25 ranking and token budget caps (GAP-CLOSURE-PLAN.md §12.4).
#[derive(Debug, Clone)]
pub struct TwoStageToolRouter {
    apps: Vec<AppCapability>,
    tools: Vec<RouteableTool>,
}

impl Default for TwoStageToolRouter {
    fn default() -> Self {
        let apps = vec![
            AppCapability {
                app_id: "ssp".to_string(),
                name: "ScreenStudio Pro".to_string(),
                one_line_summary:
                    "High-framerate desktop screen recording, display capture, and cursor tracking"
                        .to_string(),
                keywords: vec![
                    "record".into(),
                    "screen".into(),
                    "screencast".into(),
                    "capture".into(),
                    "display".into(),
                    "fps".into(),
                ],
            },
            AppCapability {
                app_id: "alp".to_string(),
                name: "Audio Labs Pro".to_string(),
                one_line_summary:
                    "Voice studio, audio mastering, dialogue isolation, and noise removal"
                        .to_string(),
                keywords: vec![
                    "audio".into(),
                    "noise".into(),
                    "denoise".into(),
                    "hum".into(),
                    "voice".into(),
                    "sound".into(),
                    "mastering".into(),
                ],
            },
            AppCapability {
                app_id: "eep".to_string(),
                name: "Easy Edits Pro".to_string(),
                one_line_summary: "Timeline video editing, interval cutting, and silence trimming"
                    .to_string(),
                keywords: vec![
                    "cut".into(),
                    "trim".into(),
                    "silence".into(),
                    "timeline".into(),
                    "video".into(),
                    "edit".into(),
                    "splice".into(),
                ],
            },
            AppCapability {
                app_id: "nwp".to_string(),
                name: "Net Watch Pro".to_string(),
                one_line_summary:
                    "Real-time network telemetry, socket inspection, and bandwidth monitoring"
                        .to_string(),
                keywords: vec![
                    "net".into(),
                    "network".into(),
                    "socket".into(),
                    "traffic".into(),
                    "bandwidth".into(),
                    "port".into(),
                ],
            },
            AppCapability {
                app_id: "atp".to_string(),
                name: "Agentic Tuneup Pro".to_string(),
                one_line_summary:
                    "System health monitoring, disk usage analysis, and package cache cleanup"
                        .to_string(),
                keywords: vec![
                    "cache".into(),
                    "clean".into(),
                    "disk".into(),
                    "tune".into(),
                    "package".into(),
                    "system".into(),
                ],
            },
        ];

        Self {
            apps,
            tools: Vec::new(),
        }
    }
}

impl TwoStageToolRouter {
    #[must_use]
    pub fn new(apps: Vec<AppCapability>, tools: Vec<RouteableTool>) -> Self {
        Self { apps, tools }
    }

    /// Adds a tool candidate to the catalog.
    pub fn add_tool(&mut self, tool: RouteableTool) {
        self.tools.push(tool);
    }

    /// Stage 1: Selects relevant apps from one-line capability list based on user goal.
    #[must_use]
    pub fn select_candidate_apps(&self, user_goal: &str) -> HashSet<String> {
        let goal_lower = user_goal.to_ascii_lowercase();
        let goal_terms = Self::tokenize(&goal_lower);

        let mut selected = HashSet::new();

        for app in &self.apps {
            let app_id_lower = app.app_id.to_ascii_lowercase();
            let app_name_lower = app.name.to_ascii_lowercase();

            // Direct mention check
            if goal_lower.contains(&app_id_lower) || goal_lower.contains(&app_name_lower) {
                selected.insert(app.app_id.clone());
                continue;
            }

            // Keyword match check
            for kw in &app.keywords {
                if goal_terms.contains(kw) || goal_lower.contains(kw) {
                    selected.insert(app.app_id.clone());
                    break;
                }
            }
        }

        // If no apps matched specific keywords, fall back to all known apps
        if selected.is_empty() {
            for app in &self.apps {
                selected.insert(app.app_id.clone());
            }
        }

        selected
    }

    /// Stage 2: Injects candidate apps' tools, ranked by BM25, strictly capped by token budget.
    #[must_use]
    pub fn route_tools(&self, user_goal: &str, mut budget: TokenBudget) -> Vec<RouteableTool> {
        // Stage 1 filter: Candidate apps
        let candidate_apps = self.select_candidate_apps(user_goal);

        // Filter tools to candidate apps
        let candidate_tools: Vec<&RouteableTool> = self
            .tools
            .iter()
            .filter(|t| candidate_apps.contains(&t.app_id))
            .collect();

        if candidate_tools.is_empty() {
            return Vec::new();
        }

        // Stage 2: Rank candidate tools with BM25
        let query_terms = Self::tokenize(&user_goal.to_ascii_lowercase());
        let scored_tools = Self::rank_bm25(&query_terms, &candidate_tools);

        // Inject tools until token budget is exhausted
        let mut injected = Vec::new();
        for (tool, _score) in scored_tools {
            let tool_json = serde_json::to_string(tool).unwrap_or_default();
            if budget.try_consume(&tool_json) {
                injected.push((*tool).clone());
            } else {
                // Out of token budget
                break;
            }
        }

        injected
    }

    /// BM25 ranking algorithm over tool names and descriptions.
    #[allow(clippy::cast_precision_loss)]
    fn rank_bm25<'a>(
        query_terms: &[String],
        tools: &[&'a RouteableTool],
    ) -> Vec<(&'a RouteableTool, f64)> {
        let n = tools.len() as f64;
        let mut doc_lens = Vec::with_capacity(tools.len());
        let mut doc_term_freqs: Vec<HashMap<String, usize>> = Vec::with_capacity(tools.len());
        let mut doc_freqs: HashMap<String, usize> = HashMap::new();

        for tool in tools {
            let text =
                format!("{} {} {}", tool.name, tool.app_id, tool.description).to_ascii_lowercase();
            let terms = Self::tokenize(&text);
            doc_lens.push(terms.len() as f64);

            let mut tf = HashMap::new();
            let mut unique_in_doc = HashSet::new();
            for term in terms {
                *tf.entry(term.clone()).or_insert(0) += 1;
                unique_in_doc.insert(term);
            }

            for term in unique_in_doc {
                *doc_freqs.entry(term).or_insert(0) += 1;
            }
            doc_term_freqs.push(tf);
        }

        let avgdl = if doc_lens.is_empty() {
            1.0
        } else {
            doc_lens.iter().sum::<f64>() / n
        };

        let k1 = 1.2;
        let b = 0.75;

        let mut scored: Vec<(&'a RouteableTool, f64)> = tools
            .iter()
            .enumerate()
            .map(|(idx, &tool)| {
                let dl = doc_lens[idx];
                let tf_map = &doc_term_freqs[idx];
                let mut score = 0.0;

                for q in query_terms {
                    if let Some(&f) = tf_map.get(q) {
                        let df = doc_freqs.get(q).copied().unwrap_or(0) as f64;
                        // Robertson-Spärck Jones IDF
                        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                        let f_f64 = f as f64;
                        let num = f_f64 * (k1 + 1.0);
                        let denom = f_f64 + k1 * (1.0 - b + b * (dl / avgdl));
                        score += idf * (num / denom);
                    }
                }

                (tool, score)
            })
            .collect();

        // Sort descending by score
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    /// Tokenizes string into clean alphanumeric words, filtering out common noise stop-words.
    fn tokenize(text: &str) -> Vec<String> {
        let stop_words: HashSet<&'static str> = [
            "the", "a", "an", "in", "to", "and", "of", "with", "for", "on", "at", "by", "from",
            "is", "my", "please", "can", "you", "i", "want",
        ]
        .iter()
        .copied()
        .collect();

        text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .filter(|s| !s.is_empty() && s.len() >= 2)
            .map(str::to_ascii_lowercase)
            .filter(|s| !stop_words.contains(s.as_str()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_stage_routing_with_500_mock_tools() {
        let mut router = TwoStageToolRouter::default();

        // 1. Generate 500 mock tools across 5 applications (100 per app)
        let apps = ["ssp", "alp", "eep", "nwp", "atp"];
        let categories = [
            "render", "export", "slice", "denoise", "gain", "filter", "network", "socket", "route",
            "cache", "disk", "clean", "format", "compress", "preview", "inspect", "snapshot",
            "backup", "restore", "sync",
        ];

        for i in 0..500 {
            let app = apps[i % apps.len()];
            let cat = categories[(i / apps.len()) % categories.len()];
            let name = format!("{app}.{cat}.action_{i}");
            let desc = format!("Performs high performance {cat} operations on {app} subsystem");
            router.add_tool(RouteableTool::new(app, name, desc, 2));
        }

        // Add 20 specific target golden tools
        let golden_cases = [
            (
                "Record 60fps desktop screen display",
                "ssp.screen.capture_60fps",
                "ssp",
                "Captures 60fps high resolution display",
            ),
            (
                "Cut silence intervals on timeline in EEP",
                "eep.timeline.cut_silence",
                "eep",
                "Precision cuts silence intervals on video timeline",
            ),
            (
                "Remove microphone hum and noise in ALP",
                "alp.audio.remove_hum",
                "alp",
                "Isolates vocal frequencies and eliminates background hum",
            ),
            (
                "Clean package cache in ATP",
                "atp.package.clean_cache",
                "atp",
                "Removes downloaded package cache to reclaim disk space",
            ),
            (
                "Inspect open network ports with NWP",
                "nwp.telemetry.inspect_ports",
                "nwp",
                "Real-time socket inspection of active listening ports",
            ),
            (
                "Export video timeline to mp4 in Easy Edits Pro",
                "eep.timeline.export_mp4",
                "eep",
                "Exports active timeline to h264 mp4 format",
            ),
            (
                "Capture cursor movement in ScreenStudio",
                "ssp.cursor.track_movement",
                "ssp",
                "Tracks smoothed cursor position during screen recording",
            ),
            (
                "Spectral denoise vocal track in Audio Labs",
                "alp.spectral.denoise_vocal",
                "alp",
                "Applies AI spectral subtraction to vocal audio tracks",
            ),
            (
                "Wipe thumbnail cache in ATP",
                "atp.cache.wipe_thumbnails",
                "atp",
                "Safely purges generated desktop thumbnail caches",
            ),
            (
                "Monitor bandwidth consumption with Net Watch",
                "nwp.bandwidth.monitor_traffic",
                "nwp",
                "Measures incoming and outgoing network interface bandwidth",
            ),
            (
                "Trim dead air in EEP timeline",
                "eep.deadair.trim_intervals",
                "eep",
                "Automated ripple trim of dead air across project timeline",
            ),
            (
                "Record window screencast at 30fps",
                "ssp.window.record_30fps",
                "ssp",
                "Captures application window video at 30 frames per second",
            ),
            (
                "Normalize audio loudness to LUFS standard",
                "alp.loudness.normalize_lufs",
                "alp",
                "Normalizes audio track loudness to -14 LUFS standard",
            ),
            (
                "Clean temporary build files in ATP",
                "atp.build.clean_temp",
                "atp",
                "Scans and deletes temporary compiler build artifacts",
            ),
            (
                "Analyze packet latency in Net Watch",
                "nwp.packet.analyze_latency",
                "nwp",
                "Monitors round-trip packet latency and network jitter",
            ),
            (
                "Splice video clips on video track 1 in EEP",
                "eep.track.splice_clips",
                "eep",
                "Splices two video clips together with crossfade transition",
            ),
            (
                "Record audio microphone with SSP",
                "ssp.microphone.record_input",
                "ssp",
                "Captures synchronized microphone audio input stream",
            ),
            (
                "Filter audio background chatter in ALP",
                "alp.voice.filter_chatter",
                "alp",
                "Isolates primary voice against background conversation chatter",
            ),
            (
                "Check disk space consumption in ATP",
                "atp.disk.check_consumption",
                "atp",
                "Scans disk mounts and calculates used storage percentages",
            ),
            (
                "Trace network socket connections in NWP",
                "nwp.socket.trace_connections",
                "nwp",
                "Traces established TCP and UDP socket connection states",
            ),
        ];

        for (_query, tool_name, app_id, desc) in golden_cases {
            router.add_tool(RouteableTool::new(app_id, tool_name, desc, 2));
        }

        // Budget: 1500 tokens (enough for ~15 tools, but far below 520 tools which would take >15,000 tokens)
        let max_budget_tokens = 1500;
        let mut found_count: usize = 0;

        for (query, expected_tool, _app_id, _desc) in golden_cases {
            let budget = TokenBudget::new(max_budget_tokens);
            let routed = router.route_tools(query, budget);

            // Assert 1: Total routed tools size strictly stays under token budget
            let total_tokens: usize = routed
                .iter()
                .map(|t| TokenBudget::estimate_tokens(&serde_json::to_string(t).unwrap()))
                .sum();
            assert!(
                total_tokens <= max_budget_tokens,
                "Routing exceeded budget: {total_tokens} > {max_budget_tokens}"
            );

            // Assert 2: Expected tool is present in the routed set
            if routed.iter().any(|t| t.name == expected_tool) {
                found_count += 1;
            }
        }

        #[allow(clippy::cast_precision_loss)]
        let accuracy_pct = (found_count as f64 / golden_cases.len() as f64) * 100.0;
        println!(
            "Two-stage tool routing accuracy with 520 tools: {found_count}/{} ({accuracy_pct:.1}%)",
            golden_cases.len()
        );

        // Verification requirement for 12.4: Target tool present in ≥ 95% of golden cases
        assert!(
            accuracy_pct >= 95.0,
            "Tool routing accuracy {accuracy_pct:.1}% below required 95% threshold"
        );
        assert_eq!(
            found_count, 20,
            "Expected all 20 golden cases to find the target tool"
        );
    }
}
