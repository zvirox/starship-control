use crate::client::{McpRequest, McpResponse, UnixTransportClient};
use async_trait::async_trait;
use serde::Deserialize;
use starship_core::CoreError;
use starship_plugin::{HelloResult, PluginTool, ProgressNotification};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tracing::debug;

pub type ProgressHandler = Box<dyn FnMut(ProgressNotification) + Send + Sync>;

/// Common abstraction over MCP transports (Unix domain socket or child process stdio).
#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<McpResponse, CoreError>;

    async fn send_request_with_progress(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
        on_progress: Option<ProgressHandler>,
    ) -> Result<McpResponse, CoreError> {
        let _ = on_progress;
        self.send_request(method, params).await
    }
}

#[async_trait]
impl McpTransport for UnixTransportClient {
    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<McpResponse, CoreError> {
        self.send_request(method, params).await
    }

    async fn send_request_with_progress(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
        on_progress: Option<ProgressHandler>,
    ) -> Result<McpResponse, CoreError> {
        self.send_request_with_progress(method, params, on_progress)
            .await
    }
}

/// Headless child process transport communicating over stdin and stdout pipes.
pub struct StdioTransportClient {
    binary_path: PathBuf,
    child: Child,
    stdin: ChildStdin,
    stdout_reader: BufReader<ChildStdout>,
    next_id: u64,
}

impl StdioTransportClient {
    /// Spawns a headless application binary with standard MCP flags (e.g. `--mcp`).
    pub fn spawn<P: AsRef<Path>, S: AsRef<str>>(
        binary_path: P,
        args: &[S],
    ) -> Result<Self, CoreError> {
        Self::spawn_with_containment(
            binary_path,
            args,
            &crate::containment::ContainmentConfig::default(),
        )
    }

    /// Spawns a headless application binary with explicit process containment settings
    /// (GAP-CLOSURE-PLAN.md §15.4).
    pub fn spawn_with_containment<P: AsRef<Path>, S: AsRef<str>>(
        binary_path: P,
        args: &[S],
        containment: &crate::containment::ContainmentConfig,
    ) -> Result<Self, CoreError> {
        let bin = binary_path.as_ref().to_path_buf();
        let str_args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();

        let (mut cmd, contained) =
            crate::containment::prepare_command(&bin, &str_args, containment);

        debug!(
            "Spawning headless MCP process: {} (contained: {}) with args: {:?}",
            bin.display(),
            contained,
            str_args
        );

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                CoreError::mcp(
                    "mcp.app_spawn_failed",
                    false,
                    format!("Failed to spawn headless process {}: {e}", bin.display()),
                    Some(serde_json::json!({
                        "binaryPath": bin.to_string_lossy(),
                        "args": str_args,
                        "contained": contained,
                    })),
                )
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            CoreError::mcp(
                "mcp.app_spawn_failed",
                false,
                "Failed to open child process stdin pipe".to_string(),
                None,
            )
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            CoreError::mcp(
                "mcp.app_spawn_failed",
                false,
                "Failed to open child process stdout pipe".to_string(),
                None,
            )
        })?;

        Ok(Self {
            binary_path: bin,
            child,
            stdin,
            stdout_reader: BufReader::new(stdout),
            next_id: 1,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}

#[async_trait]
impl McpTransport for StdioTransportClient {
    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<McpResponse, CoreError> {
        self.send_request_with_progress(method, params, None).await
    }

    async fn send_request_with_progress(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
        mut on_progress: Option<ProgressHandler>,
    ) -> Result<McpResponse, CoreError> {
        let req_id = self.next_id;
        self.next_id += 1;

        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: req_id,
            method: method.to_string(),
            params,
        };

        let mut payload = serde_json::to_string(&req).map_err(|e| {
            CoreError::mcp(
                "mcp.serialization_error",
                false,
                format!("Failed to serialize request: {e}"),
                None,
            )
        })?;
        payload.push('\n');

        self.stdin
            .write_all(payload.as_bytes())
            .await
            .map_err(|e| {
                CoreError::mcp(
                    "mcp.socket_unreachable",
                    true,
                    format!(
                        "Failed to write to child stdin {}: {e}",
                        self.binary_path.display()
                    ),
                    None,
                )
            })?;

        self.stdin.flush().await.map_err(|e| {
            CoreError::mcp(
                "mcp.socket_unreachable",
                true,
                format!(
                    "Failed to flush child stdin {}: {e}",
                    self.binary_path.display()
                ),
                None,
            )
        })?;

        loop {
            let mut line = String::new();
            self.stdout_reader.read_line(&mut line).await.map_err(|e| {
                CoreError::mcp(
                    "mcp.socket_unreachable",
                    true,
                    format!(
                        "Failed to read from child stdout {}: {e}",
                        self.binary_path.display()
                    ),
                    None,
                )
            })?;

            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Err(CoreError::mcp(
                    "mcp.socket_unreachable",
                    true,
                    "Child process closed stdout pipe unexpectedly".to_string(),
                    None,
                ));
            }

            // Check if this line is an intermediate progress notification (GAP-CLOSURE-PLAN.md §13.1)
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(m) = value.get("method").and_then(serde_json::Value::as_str)
                    && m == "notifications/progress"
                {
                    if let Some(params_val) = value.get("params")
                        && let Ok(prog) =
                            serde_json::from_value::<ProgressNotification>(params_val.clone())
                        && let Some(ref mut cb) = on_progress
                    {
                        cb(prog);
                    }
                    continue;
                }

                if let Some(id) = value.get("id").and_then(serde_json::Value::as_u64)
                    && id == req_id
                {
                    let resp: McpResponse = serde_json::from_value(value).map_err(|e| {
                        CoreError::mcp(
                            "mcp.deserialization_error",
                            false,
                            format!("Failed to deserialize response from child: {e}"),
                            Some(serde_json::json!({ "raw": trimmed })),
                        )
                    })?;
                    return Ok(resp);
                }
            }

            let resp: McpResponse = serde_json::from_str(trimmed).map_err(|e| {
                CoreError::mcp(
                    "mcp.deserialization_error",
                    false,
                    format!("Failed to deserialize response from child: {e}"),
                    Some(serde_json::json!({ "raw": trimmed })),
                )
            })?;

            if resp.id == req_id {
                return Ok(resp);
            }
        }
    }
}

impl Drop for StdioTransportClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

#[derive(Deserialize)]
struct ListResult {
    tools: Vec<PluginTool>,
}

/// A high-level session over any active MCP transport.
pub struct McpSession {
    transport: Box<dyn McpTransport>,
}

impl McpSession {
    #[must_use]
    pub fn new(transport: Box<dyn McpTransport>) -> Self {
        Self { transport }
    }

    /// Performs handshake negotiation (`starship/hello`).
    pub async fn handshake(&mut self) -> Result<HelloResult, CoreError> {
        let params = serde_json::json!({
            "starshipVersion": "1.0.0",
            "supportedMcpVersion": "2024-11-05"
        });

        let resp = self
            .transport
            .send_request("starship/hello", Some(params))
            .await?;

        if let Some(err) = resp.error {
            return Err(CoreError::mcp(
                "mcp.handshake_timeout",
                false,
                format!("Handshake failed: {err}"),
                Some(err),
            ));
        }

        let res_val = resp.result.ok_or_else(|| {
            CoreError::mcp(
                "mcp.handshake_timeout",
                false,
                "Handshake response missing result payload".to_string(),
                None,
            )
        })?;

        serde_json::from_value(res_val).map_err(|e| {
            CoreError::mcp(
                "mcp.handshake_timeout",
                false,
                format!("Invalid handshake structure: {e}"),
                None,
            )
        })
    }

    /// Queries the list of exposed tools (`tools/list`).
    pub async fn list_tools(&mut self) -> Result<Vec<PluginTool>, CoreError> {
        let resp = self.transport.send_request("tools/list", None).await?;

        if let Some(err) = resp.error {
            return Err(CoreError::mcp(
                "mcp.execution_failed",
                false,
                format!("tools/list failed: {err}"),
                Some(err),
            ));
        }

        let res_val = resp.result.ok_or_else(|| {
            CoreError::mcp(
                "mcp.execution_failed",
                false,
                "tools/list response missing result payload".to_string(),
                None,
            )
        })?;

        let parsed: ListResult = serde_json::from_value(res_val).map_err(|e| {
            CoreError::mcp(
                "mcp.execution_failed",
                false,
                format!("Invalid tools/list format: {e}"),
                None,
            )
        })?;

        Ok(parsed.tools)
    }

    /// Executes an MCP tool call with lease timeout (`tools/call`).
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
    ) -> Result<serde_json::Value, CoreError> {
        self.call_tool_with_progress(tool_name, arguments, timeout_ms, None, None)
            .await
    }

    /// Executes an MCP tool call with optional `progressToken` and streaming progress callback (GAP-CLOSURE-PLAN.md §13.1).
    pub async fn call_tool_with_progress(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
        timeout_ms: u64,
        progress_token: Option<serde_json::Value>,
        on_progress: Option<ProgressHandler>,
    ) -> Result<serde_json::Value, CoreError> {
        let mut params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments,
            "leaseTimeoutMs": timeout_ms,
        });

        if let Some(ref token) = progress_token {
            params["progressToken"] = token.clone();
            params["_meta"] = serde_json::json!({
                "progressToken": token
            });
        }

        let call_future =
            self.transport
                .send_request_with_progress("tools/call", Some(params), on_progress);
        let resp = tokio::time::timeout(Duration::from_millis(timeout_ms), call_future)
            .await
            .map_err(|_| {
                CoreError::mcp(
                    "mcp.execution_failed",
                    true,
                    format!("Tool call '{tool_name}' timed out after {timeout_ms}ms"),
                    Some(serde_json::json!({
                        "toolName": tool_name,
                        "timeoutMs": timeout_ms,
                    })),
                )
            })??;

        if let Some(err) = resp.error {
            return Err(CoreError::mcp(
                "mcp.execution_failed",
                false,
                format!("Tool execution error: {err}"),
                Some(err),
            ));
        }

        resp.result.ok_or_else(|| {
            CoreError::mcp(
                "mcp.execution_failed",
                false,
                "Tool call returned empty result".to_string(),
                None,
            )
        })
    }

    /// Cancels an in-flight tool call (`tools/cancel`).
    pub async fn cancel_tool(&mut self, call_id: u64, reason: &str) -> Result<(), CoreError> {
        let params = serde_json::json!({
            "callId": call_id,
            "reason": reason,
        });

        let _ = self
            .transport
            .send_request("tools/cancel", Some(params))
            .await;
        Ok(())
    }
}
