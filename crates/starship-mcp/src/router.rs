use crate::client::UnixTransportClient;
use crate::protocol::{McpSession, StdioTransportClient};
use starship_core::CoreError;
use starship_db::Database;
use starship_plugin::bridge_dir;
use std::path::{Path, PathBuf};
use tracing::info;

/// Hybrid Execution Router dispatching MCP commands across Live GUI sockets or Headless CLI stdio pipes.
#[derive(Clone)]
pub struct HybridExecutionRouter {
    db: Database,
}

impl HybridExecutionRouter {
    #[must_use]
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Dispatches a tool execution to the appropriate transport (Socket vs Stdio vs Simulation).
    pub async fn route_and_call(
        &self,
        app_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, CoreError> {
        // 1. Check for Active GUI Socket: $XDG_RUNTIME_DIR/starship/bridges/<app-id>.sock
        if let Some(path) = self.get_active_socket_path(app_id) {
            info!(
                "Routing tool call '{tool_name}' via Live GUI socket: {}",
                path.display()
            );
            return self
                .call_via_socket(&path, tool_name, arguments, timeout_ms)
                .await;
        }

        // 2. Check for Installed CLI Binary: <binaryPath> --mcp
        if let Ok(Some(app)) = self.db.get_app(app_id) {
            let bin_path = PathBuf::from(&app.binary_path);
            if bin_path.exists() {
                // Enforce manifest pinning & path safety (GAP-CLOSURE-PLAN.md §10.5)
                crate::security::SecurityVerifier::validate_binary_path(&bin_path)?;

                info!(
                    "Routing tool call '{tool_name}' via Headless Stdio pipe: {}",
                    bin_path.display()
                );
                return self
                    .call_via_stdio(&bin_path, tool_name, arguments, timeout_ms)
                    .await;
            }
        }

        // 3. No real transport available — never invent a successful result.
        let mut checked = vec![
            bridge_dir()
                .join(format!("{app_id}.sock"))
                .to_string_lossy()
                .to_string(),
        ];
        if let Ok(Some(app)) = self.db.get_app(app_id) {
            if let Some(sock) = app.socket_path {
                let path = PathBuf::from(&sock).to_string_lossy().to_string();
                if !checked.contains(&path) {
                    checked.push(path);
                }
            }
            let binary = PathBuf::from(&app.binary_path)
                .to_string_lossy()
                .to_string();
            if !checked.contains(&binary) {
                checked.push(binary);
            }
        }
        Err(CoreError::mcp(
            "mcp.app_unavailable",
            false,
            format!("Application '{app_id}' is not available (no bridge socket or usable binary)"),
            Some(serde_json::json!({
                "appId": app_id,
                "tool": tool_name,
                "checked": checked,
                "reason": "no_active_socket_or_binary",
            })),
        ))
    }

    fn get_active_socket_path(&self, app_id: &str) -> Option<PathBuf> {
        let default_sock = bridge_dir().join(format!("{app_id}.sock"));
        if default_sock.exists() {
            return Some(default_sock);
        }

        if let Ok(Some(app)) = self.db.get_app(app_id)
            && let Some(sock_str) = app.socket_path
        {
            let p = PathBuf::from(sock_str);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }

    async fn call_via_socket(
        &self,
        socket_path: &Path,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, CoreError> {
        let client = UnixTransportClient::connect(socket_path).await?;
        let mut session = McpSession::new(Box::new(client));
        session.call_tool(tool_name, arguments, timeout_ms).await
    }

    async fn call_via_stdio(
        &self,
        binary_path: &Path,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, CoreError> {
        let client = StdioTransportClient::spawn(binary_path, &["--mcp"])?;
        let mut session = McpSession::new(Box::new(client));
        session.call_tool(tool_name, arguments, timeout_ms).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_no_transport_returns_typed_error_not_simulated_success() {
        let router = HybridExecutionRouter::new(Database::open_in_memory().expect("in-memory db"));
        let err = router
            .route_and_call("missing-app", "anything.do", serde_json::json!({}), 1_000)
            .await
            .expect_err("no transport must fail");
        assert_eq!(err.code, "mcp.app_unavailable");
        assert!(!err.retryable);
        let details = err.details.expect("error details");
        assert_eq!(details["appId"], "missing-app");
        assert_eq!(details["tool"], "anything.do");
        assert_eq!(details["reason"], "no_active_socket_or_binary");
        assert!(
            details["checked"]
                .as_array()
                .is_some_and(|paths| !paths.is_empty())
        );
    }
}
