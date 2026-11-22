use crate::CoreError;
use crate::planner::tasksheet::TaskSheet;

/// Builds the system prompt and planning template instructing an LLM
/// to decompose a user mission into a canonical `tasksheet/1` DAG.
pub struct PlannerPromptBuilder;

impl PlannerPromptBuilder {
    /// Constructs the decomposition prompt with untrusted context wrapping (`AGENTS.md §3.5`).
    #[must_use]
    pub fn build_prompt(
        user_goal: &str,
        available_tools_json: &str,
        external_context: Option<&str>,
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str(
            "You are Starship Control Planner, a local-first autonomous orchestrator for Linux creative desktop applications.\n\
             Your duty is to decompose user strategic goals into a directed acyclic graph (DAG) execution plan in tasksheet/1 format.\n\n\
             ### Available Applications & Tools:\n",
        );
        prompt.push_str(available_tools_json);
        prompt.push_str("\n\n");

        if let Some(ctx) = external_context {
            prompt.push_str("### Context Information:\n<untrusted_context>\n");
            prompt.push_str(ctx);
            prompt.push_str("\n</untrusted_context>\n\n");
        }

        prompt.push_str(
            "### Task Sheet Specification (tasksheet/1):\n\
             Output ONLY valid JSON adhering strictly to this schema:\n\
             {\n\
               \"version\": 1,\n\
               \"goal\": \"<original user goal>\",\n\
               \"title\": \"<short title of the workflow>\",\n\
               \"summary\": \"<plain english summary of what this plan achieves>\",\n\
               \"steps\": [\n\
                 {\n\
                   \"step_index\": 0,\n\
                   \"app_id\": \"<app_id, e.g. ssp, alp, eep, nwp, atp>\",\n\
                   \"tool_name\": \"<tool identifier, e.g. screen.record, audio.cleanup>\",\n\
                   \"arguments\": { ... },\n\
                   \"risk_tier\": 1,\n\
                   \"diff_summary\": \"<human readable description of what changes or inspects>\",\n\
                   \"depends_on\": []\n\
                 }\n\
               ]\n\
             }\n\n\
             ### Risk Tier Guidelines:\n\
             - Tier 1 (Safe / Autonomous): Read-only status checks, metric queries, inspection tools.\n\
             - Tier 2 (Mutating / Approval Gate): Applying edits, modifying audio/video tracks, applying filters, synthesizing audio.\n\
             - Tier 3 (Destructive / High-Stakes): File deletions, cache purges, irreversible overwrites.\n\n\
             ### User Strategic Goal:\n<untrusted_context>\n",
        );
        prompt.push_str(user_goal);
        prompt.push_str("\n</untrusted_context>\n\nOutput the JSON plan:");

        prompt
    }

    /// Extracts and parses a `TaskSheet` from raw LLM output,
    /// safely stripping markdown code blocks if present.
    pub fn parse_llm_plan(raw_output: &str) -> Result<TaskSheet, CoreError> {
        let trimmed = raw_output.trim();

        // Check if output is wrapped in ```json ... ``` or ``` ... ```
        let json_str = if let Some(stripped) = trimmed.strip_prefix("```json") {
            stripped.strip_suffix("```").unwrap_or(stripped).trim()
        } else if let Some(stripped) = trimmed.strip_prefix("```") {
            stripped.strip_suffix("```").unwrap_or(stripped).trim()
        } else if let Some(start_idx) = trimmed.find('{') {
            if let Some(end_idx) = trimmed.rfind('}') {
                if end_idx >= start_idx {
                    &trimmed[start_idx..=end_idx]
                } else {
                    trimmed
                }
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        TaskSheet::from_llm_json(json_str)
    }
}
