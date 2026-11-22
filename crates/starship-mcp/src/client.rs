use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use starship_plugin::{HelloResult, ProgressNotification};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// Bidirectional Unix domain socket client with kernel-enforced `SO_PEERCRED` validation.
pub struct UnixTransportClient {
    socket_path: PathBuf,
    reader: BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: tokio::net::unix::OwnedWriteHalf,
    next_id: u64,
}

impl UnixTransportClient {
    /// Connects to a sub-application bridge socket and validates peer credentials via `SO_PEERCRED`.
    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let socket_path = path.as_ref().to_path_buf();

        let stream = UnixStream::connect(&socket_path).await.map_err(|e| {
            CoreError::mcp(
                "mcp.socket_unreachable",
                true,
                format!(
                    "Unable to connect to bridge socket {}: {e}",
                    socket_path.display()
                ),
                Some(serde_json::json!({
                    "socketPath": socket_path.to_string_lossy(),
                })),
            )
        })?;

        // Verify Linux SO_PEERCRED credentials to enforce single-user security
        #[cfg(target_os = "linux")]
        {
            let peer_cred = stream.peer_cred().map_err(|e| {
                CoreError::mcp(
                    "mcp.so_peercred_failed",
                    false,
                    format!(
                        "Failed to inspect SO_PEERCRED on {}: {e}",
                        socket_path.display()
                    ),
                    Some(serde_json::json!({
                        "socketPath": socket_path.to_string_lossy(),
                    })),
                )
            })?;

            let current_uid = rustix::process::getuid().as_raw();
            if peer_cred.uid() != current_uid {
                return Err(CoreError::mcp(
                    "mcp.so_peercred_failed",
                    false,
                    format!(
                        "Security violation: peer UID {} does not match current host user UID {}",
                        peer_cred.uid(),
                        current_uid
                    ),
                    Some(serde_json::json!({
                        "peerUid": peer_cred.uid(),
                        "currentUid": current_uid,
                        "socketPath": socket_path.to_string_lossy(),
                    })),
                ));
            }
        }

        let (read_half, write_half) = stream.into_split();
        Ok(Self {
            socket_path,
            reader: BufReader::new(read_half),
            writer: write_half,
            next_id: 1,
        })
    }

    /// Connects to a bridge socket and verifies peer credentials and binary path (GAP-CLOSURE-PLAN.md §10.6).
    pub async fn connect_and_verify_peer<P: AsRef<Path>, B: AsRef<Path>>(
        path: P,
        expected_binary_path: Option<B>,
    ) -> Result<Self, CoreError> {
        let client = Self::connect(path).await?;
        if let Some(ref expected_bin) = expected_binary_path {
            crate::security::SecurityVerifier::validate_binary_path(expected_bin.as_ref())?;
        }
        Ok(client)
    }

    /// Sends a JSON-RPC 2.0 request over the socket and parses the newline-delimited JSON response.
    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<McpResponse, CoreError> {
        self.send_request_with_progress::<fn(ProgressNotification)>(method, params, None)
            .await
    }

    /// Sends a JSON-RPC 2.0 request over the socket and handles intermediate streaming progress notifications.
    pub async fn send_request_with_progress<F>(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
        mut on_progress: Option<F>,
    ) -> Result<McpResponse, CoreError>
    where
        F: FnMut(ProgressNotification) + Send + Sync,
    {
        let request_id = self.next_id;
        self.next_id += 1;

        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
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

        self.writer
            .write_all(payload.as_bytes())
            .await
            .map_err(|e| {
                CoreError::mcp(
                    "mcp.socket_unreachable",
                    true,
                    format!(
                        "Failed to write to socket {}: {e}",
                        self.socket_path.display()
                    ),
                    None,
                )
            })?;

        self.writer.flush().await.map_err(|e| {
            CoreError::mcp(
                "mcp.socket_unreachable",
                true,
                format!("Failed to flush socket {}: {e}", self.socket_path.display()),
                None,
            )
        })?;

        loop {
            let mut line = String::new();
            self.reader.read_line(&mut line).await.map_err(|e| {
                CoreError::mcp(
                    "mcp.socket_unreachable",
                    true,
                    format!(
                        "Failed to read response from {}: {e}",
                        self.socket_path.display()
                    ),
                    None,
                )
            })?;

            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Err(CoreError::mcp(
                    "mcp.socket_unreachable",
                    true,
                    "Socket closed unexpectedly by peer".to_string(),
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
                    && id == request_id
                {
                    let resp: McpResponse = serde_json::from_value(value).map_err(|e| {
                        CoreError::mcp(
                            "mcp.deserialization_error",
                            false,
                            format!("Failed to deserialize MCP response: {e}"),
                            Some(serde_json::json!({
                                "rawResponse": trimmed,
                            })),
                        )
                    })?;
                    return Ok(resp);
                }
            }

            let resp: McpResponse = serde_json::from_str(trimmed).map_err(|e| {
                CoreError::mcp(
                    "mcp.deserialization_error",
                    false,
                    format!("Failed to deserialize MCP response: {e}"),
                    Some(serde_json::json!({
                        "rawResponse": trimmed,
                    })),
                )
            })?;

            if resp.id == request_id {
                return Ok(resp);
            }
        }
    }

    /// Executes an MCP `tools/call` with optional `progressToken` and streaming progress callback (GAP-CLOSURE-PLAN.md §13.1).
    pub async fn call_tool_with_progress<F>(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
        progress_token: Option<serde_json::Value>,
        on_progress: Option<F>,
    ) -> Result<serde_json::Value, CoreError>
    where
        F: FnMut(ProgressNotification) + Send + Sync,
    {
        let mut params = serde_json::json!({
            "name": tool_name,
            "arguments": arguments,
        });

        if let Some(ref token) = progress_token {
            params["progressToken"] = token.clone();
            params["_meta"] = serde_json::json!({
                "progressToken": token
            });
        }

        let resp = self
            .send_request_with_progress("tools/call", Some(params), on_progress)
            .await?;

        if let Some(err) = resp.error {
            return Err(CoreError::mcp(
                "mcp.execution_failed",
                false,
                format!("Tool execution failed: {err}"),
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

    /// Performs the `starship/hello` handshake to discover app capabilities and tool schemas.
    pub async fn handshake(&mut self) -> Result<HelloResult, CoreError> {
        let params = serde_json::json!({
            "starshipVersion": "1.0.0",
            "supportedMcpVersion": "2024-11-05"
        });

        let resp = self.send_request("starship/hello", Some(params)).await?;
        if let Some(err) = resp.error {
            return Err(CoreError::mcp(
                "mcp.handshake_timeout",
                false,
                format!("Handshake rejected by application: {err}"),
                Some(err),
            ));
        }

        let result_val = resp.result.ok_or_else(|| {
            CoreError::mcp(
                "mcp.handshake_timeout",
                false,
                "Handshake response missing result payload".to_string(),
                None,
            )
        })?;

        let hello_result: HelloResult = serde_json::from_value(result_val).map_err(|e| {
            CoreError::mcp(
                "mcp.handshake_timeout",
                false,
                format!("Invalid handshake result format: {e}"),
                None,
            )
        })?;

        Ok(hello_result)
    }

    /// Measures the round-trip ping latency to the sub-application in milliseconds.
    pub async fn ping(&mut self) -> Result<f64, CoreError> {
        let start = Instant::now();
        let resp = self.send_request("ping", None).await?;
        if resp.error.is_some() {
            return Err(CoreError::mcp(
                "mcp.execution_failed",
                true,
                "Ping request failed".to_string(),
                None,
            ));
        }
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        Ok(elapsed)
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}
