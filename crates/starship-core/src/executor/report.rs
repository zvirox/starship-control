use crate::error::{CoreError, ErrorKind};
use crate::executor::failure::FailureClass;
use crate::executor::fallback::FallbackRung;
use crate::safety::RiskTier;
use crate::safety::secrets::SecretScrubber;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// Summary of a successfully completed step prior to failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedStepSummary {
    pub step_id: String,
    pub tool_name: String,
    pub risk_tier: RiskTier,
    pub duration_ms: u64,
}

/// Summary of a rollback operation executed during ladder traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackSummary {
    pub resource: String,
    pub action: String,
    pub status: String,
}

/// Structured failure report model per GAP-CLOSURE-PLAN.md §11.11.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureReport {
    pub mission_id: i64,
    pub mission_title: String,
    pub created_at: String,
    pub failed_at: String,
    pub failure_class: FailureClass,
    pub error_code: String,
    pub error_message: String,
    pub failed_step_id: String,
    pub failed_tool_name: String,
    pub terminal_rung: FallbackRung,
    pub steps_completed: Vec<CompletedStepSummary>,
    pub rollbacks_executed: Vec<RollbackSummary>,
    pub current_state_summary: String,
    pub suggested_manual_steps: Vec<String>,
}

impl FailureReport {
    /// Renders the failure report as standard Markdown `mission-<id>-report.md`.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        let _ = writeln!(md, "# Mission Failure Report: mission-{}", self.mission_id);
        md.push('\n');
        let _ = writeln!(md, "- **Mission Title:** {}", self.mission_title);
        let _ = writeln!(md, "- **Mission ID:** `{}`", self.mission_id);
        md.push_str("- **Status:** `FAILED`\n");
        let _ = writeln!(md, "- **Failed At:** {}", self.failed_at);
        let _ = writeln!(md, "- **Failure Class:** `{:?}`", self.failure_class);
        let _ = writeln!(md, "- **Error Code:** `{}`", self.error_code);
        let _ = writeln!(
            md,
            "- **Terminal Ladder Rung:** `{:?}`\n",
            self.terminal_rung
        );
        md.push_str("---\n\n");

        // Section 1: What Ran
        md.push_str("## What Ran\n\n");
        if self.steps_completed.is_empty() {
            md.push_str("No steps completed successfully before failure.\n\n");
        } else {
            md.push_str("| Step ID | Tool Name | Risk Tier | Duration |\n");
            md.push_str("|---|---|---|---|\n");
            for step in &self.steps_completed {
                let _ = writeln!(
                    md,
                    "| `{}` | `{}` | `{:?}` | {} ms |",
                    step.step_id, step.tool_name, step.risk_tier, step.duration_ms
                );
            }
            md.push('\n');
        }

        // Section 2: What Failed
        md.push_str("## What Failed\n\n");
        let _ = writeln!(md, "- **Triggering Step:** `{}`", self.failed_step_id);
        let _ = writeln!(md, "- **Target Tool:** `{}`", self.failed_tool_name);
        let _ = writeln!(md, "- **Error Code:** `{}`", self.error_code);
        let _ = writeln!(md, "- **Details:** {}\n", self.error_message);

        // Section 3: What Was Rolled Back
        md.push_str("## What Was Rolled Back\n\n");
        if self.rollbacks_executed.is_empty() {
            md.push_str("No mutations occurred; no rollback was required.\n\n");
        } else {
            md.push_str("| Resource | Action Taken | Status |\n");
            md.push_str("|---|---|---|\n");
            for rb in &self.rollbacks_executed {
                let _ = writeln!(
                    md,
                    "| `{}` | {} | `{}` |",
                    rb.resource, rb.action, rb.status
                );
            }
            md.push('\n');
        }

        // Section 4: Current State
        md.push_str("## Current State\n\n");
        md.push_str(&self.current_state_summary);
        md.push_str("\n\n");

        // Section 5: Suggested Manual Steps
        md.push_str("## Suggested Manual Steps\n\n");
        if self.suggested_manual_steps.is_empty() {
            md.push_str("1. Check application logs for further diagnostics.\n");
            md.push_str("2. Verify sub-application process status.\n");
        } else {
            for (idx, step) in self.suggested_manual_steps.iter().enumerate() {
                let _ = writeln!(md, "{}. {}", idx + 1, step);
            }
        }
        md.push('\n');

        md
    }
}

/// Self-contained handoff bundle containing sanitized report, plan, events, and logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandoffBundle {
    pub mission_id: i64,
    pub report_markdown: String,
    pub plan_json: String,
    pub events_json: String,
    pub logs_text: String,
    pub exported_at: String,
}

impl HandoffBundle {
    /// Creates a handoff bundle, strictly scrubbing all known secrets and API keys (GAP-CLOSURE-PLAN.md §11.11).
    #[must_use]
    pub fn create(
        report: &FailureReport,
        plan_json: &str,
        events_json: &str,
        logs_text: &str,
        known_secrets: &[&str],
    ) -> Self {
        let raw_report_md = report.to_markdown();
        let report_markdown = SecretScrubber::scrub(&raw_report_md, known_secrets);
        let scrubbed_plan = SecretScrubber::scrub(plan_json, known_secrets);
        let scrubbed_events = SecretScrubber::scrub(events_json, known_secrets);
        let scrubbed_logs = SecretScrubber::scrub(logs_text, known_secrets);

        Self {
            mission_id: report.mission_id,
            report_markdown,
            plan_json: scrubbed_plan,
            events_json: scrubbed_events,
            logs_text: scrubbed_logs,
            exported_at: report.failed_at.clone(),
        }
    }

    /// Verifies whether any of the bundle's contents contain the specified plaintext secret.
    #[must_use]
    pub fn contains_secret(&self, secret: &str) -> bool {
        let trimmed = secret.trim();
        if trimmed.is_empty() {
            return false;
        }
        self.report_markdown.contains(trimmed)
            || self.plan_json.contains(trimmed)
            || self.events_json.contains(trimmed)
            || self.logs_text.contains(trimmed)
    }

    /// Exports the scrubbed bundle files to a destination directory on disk.
    pub fn export_to_directory(&self, base_dir: &Path) -> Result<PathBuf, CoreError> {
        let bundle_dir = base_dir.join(format!("mission-{}-handoff", self.mission_id));
        fs::create_dir_all(&bundle_dir).map_err(|err| {
            CoreError::new(
                ErrorKind::App,
                "app.export_failed",
                false,
                format!("Failed to create bundle directory: {err}"),
                None,
            )
        })?;

        // 1. Write report.md
        let report_path = bundle_dir.join(format!("mission-{}-report.md", self.mission_id));
        fs::write(&report_path, &self.report_markdown).map_err(|err| {
            CoreError::new(
                ErrorKind::App,
                "app.export_failed",
                false,
                format!("Failed to write report: {err}"),
                None,
            )
        })?;

        // 2. Write plan.json
        let plan_path = bundle_dir.join("plan.json");
        fs::write(&plan_path, &self.plan_json).map_err(|err| {
            CoreError::new(
                ErrorKind::App,
                "app.export_failed",
                false,
                format!("Failed to write plan.json: {err}"),
                None,
            )
        })?;

        // 3. Write events.json
        let events_path = bundle_dir.join("events.json");
        fs::write(&events_path, &self.events_json).map_err(|err| {
            CoreError::new(
                ErrorKind::App,
                "app.export_failed",
                false,
                format!("Failed to write events.json: {err}"),
                None,
            )
        })?;

        // 4. Write logs.txt
        let logs_path = bundle_dir.join("logs.txt");
        fs::write(&logs_path, &self.logs_text).map_err(|err| {
            CoreError::new(
                ErrorKind::App,
                "app.export_failed",
                false,
                format!("Failed to write logs.txt: {err}"),
                None,
            )
        })?;

        Ok(bundle_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_failure_report_markdown_and_scrubbed_handoff_bundle() {
        let canary_secret = "sk-live-canary-key-999888777666555444333";
        let custom_secret = "custom_super_secret_db_pass_12345";

        let report = FailureReport {
            mission_id: 42,
            mission_title: "Automated Screen Recording & Denoise".to_string(),
            created_at: "2026-11-22T12:00:00Z".to_string(),
            failed_at: "2026-11-22T12:05:00Z".to_string(),
            failure_class: FailureClass::AppFailure,
            error_code: "mcp.execution_failed".to_string(),
            error_message: format!(
                "Command failed with auth token Bearer {canary_secret} and pass {custom_secret}"
            ),
            failed_step_id: "step-denoise".to_string(),
            failed_tool_name: "alp.denoise_neural".to_string(),
            terminal_rung: FallbackRung::F6SafeStop,
            steps_completed: vec![CompletedStepSummary {
                step_id: "step-rec".to_string(),
                tool_name: "ssp.record_screen".to_string(),
                risk_tier: RiskTier::Safe,
                duration_ms: 2500,
            }],
            rollbacks_executed: vec![RollbackSummary {
                resource: "alp:session:track_1".to_string(),
                action: "Restored checkpoint chk-101".to_string(),
                status: "Success".to_string(),
            }],
            current_state_summary: "All resource locks released. Sub-process terminated."
                .to_string(),
            suggested_manual_steps: vec![
                "Check that Audio Labs Pro is running on socket /tmp/alp.sock".to_string(),
                "Manually verify audio take integrity in Audio Labs Pro".to_string(),
            ],
        };

        let raw_plan =
            format!(r#"{{"steps": [{{"id": "step-denoise", "api_key": "{canary_secret}"}}]}}"#);
        let raw_events =
            format!(r#"[{{ "event": "step_failed", "details": "secret={custom_secret}" }}]"#);
        let raw_logs =
            format!("[2026-11-22 12:05:00] ERROR step-denoise: failed with key {canary_secret}\n");

        let bundle = HandoffBundle::create(
            &report,
            &raw_plan,
            &raw_events,
            &raw_logs,
            &[custom_secret, canary_secret],
        );

        // Security check: Canary secret MUST NOT appear anywhere in bundle
        assert!(
            !bundle.contains_secret(canary_secret),
            "Security vulnerability: bundle contains canary secret!"
        );
        assert!(
            !bundle.contains_secret(custom_secret),
            "Security vulnerability: bundle contains custom secret!"
        );

        // Verify Markdown structure contains all 5 required sections
        let md = &bundle.report_markdown;
        assert!(md.contains("# Mission Failure Report: mission-42"));
        assert!(md.contains("## What Ran"));
        assert!(md.contains("## What Failed"));
        assert!(md.contains("## What Was Rolled Back"));
        assert!(md.contains("## Current State"));
        assert!(md.contains("## Suggested Manual Steps"));
        assert!(md.contains("[REDACTED_API_KEY]"));

        // Verify filesystem export
        let tmp = tempdir().unwrap();
        let export_path = bundle.export_to_directory(tmp.path()).unwrap();
        assert!(export_path.exists());

        let exported_report = fs::read_to_string(export_path.join("mission-42-report.md")).unwrap();
        let exported_plan = fs::read_to_string(export_path.join("plan.json")).unwrap();
        let exported_events = fs::read_to_string(export_path.join("events.json")).unwrap();
        let exported_logs = fs::read_to_string(export_path.join("logs.txt")).unwrap();

        assert!(!exported_report.contains(canary_secret));
        assert!(!exported_plan.contains(canary_secret));
        assert!(!exported_events.contains(canary_secret));
        assert!(!exported_logs.contains(canary_secret));
        assert!(!exported_report.contains(custom_secret));
        assert!(!exported_plan.contains(custom_secret));
        assert!(!exported_events.contains(custom_secret));
        assert!(!exported_logs.contains(custom_secret));
    }
}
