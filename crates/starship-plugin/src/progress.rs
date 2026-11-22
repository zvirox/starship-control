use serde::{Deserialize, Serialize};

/// Progress notification payload matching MCP standard (`notifications/progress`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotification {
    pub progress_token: serde_json::Value,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Helper for application plugins to emit streaming progress updates (GAP-CLOSURE-PLAN.md §13.1).
#[derive(Debug, Clone)]
pub struct ProgressReporter {
    progress_token: serde_json::Value,
}

impl ProgressReporter {
    /// Creates a new `ProgressReporter` for the given progress token.
    #[must_use]
    pub fn new(progress_token: serde_json::Value) -> Self {
        Self { progress_token }
    }

    /// Extracts `progressToken` from tool call parameters (either top-level or under `_meta`).
    #[must_use]
    pub fn from_params(params: &serde_json::Value) -> Option<Self> {
        let token = params
            .get("progressToken")
            .or_else(|| params.get("_meta").and_then(|m| m.get("progressToken")));
        token.cloned().map(|t| Self { progress_token: t })
    }

    /// Returns the progress token.
    #[must_use]
    pub fn progress_token(&self) -> &serde_json::Value {
        &self.progress_token
    }

    /// Formats a JSON-RPC 2.0 `notifications/progress` notification newline-delimited string.
    #[must_use]
    pub fn format_notification(
        &self,
        progress: f64,
        total: Option<f64>,
        message: Option<&str>,
    ) -> String {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": self.progress_token,
                "progress": progress,
                "total": total,
                "message": message,
            }
        });
        format!("{payload}\n")
    }

    /// Emits a progress update synchronously over any `std::io::Write` target.
    pub fn report_sync<W: std::io::Write>(
        &self,
        writer: &mut W,
        progress: f64,
        total: Option<f64>,
        message: Option<&str>,
    ) -> std::io::Result<()> {
        let line = self.format_notification(progress, total, message);
        writer.write_all(line.as_bytes())?;
        writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_reporter_format_and_parse() {
        let reporter = ProgressReporter::new(serde_json::json!("step-42"));
        let line = reporter.format_notification(0.5, Some(1.0), Some("Halfway done"));

        let val: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(val["method"], "notifications/progress");
        assert_eq!(val["params"]["progressToken"], "step-42");
        assert_eq!(val["params"]["progress"], 0.5);
        assert_eq!(val["params"]["total"], 1.0);
        assert_eq!(val["params"]["message"], "Halfway done");
    }

    #[test]
    fn test_progress_reporter_from_params() {
        let params_top = serde_json::json!({
            "progressToken": 123
        });
        let reporter1 = ProgressReporter::from_params(&params_top).unwrap();
        assert_eq!(*reporter1.progress_token(), 123);

        let params_meta = serde_json::json!({
            "_meta": {
                "progressToken": "token-abc"
            }
        });
        let reporter2 = ProgressReporter::from_params(&params_meta).unwrap();
        assert_eq!(*reporter2.progress_token(), "token-abc");
    }
}
