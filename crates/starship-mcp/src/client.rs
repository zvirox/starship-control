use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use starship_plugin::HelloResult;
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

    /// Sends a JSON-RPC 2.0 request over the socket and parses the newline-delimited JSON response.
    pub async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<McpResponse, CoreError> {
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

        if line.trim().is_empty() {
            return Err(CoreError::mcp(
                "mcp.socket_unreachable",
                true,
                "Socket closed unexpectedly by peer".to_string(),
                None,
            ));
        }

        let resp: McpResponse = serde_json::from_str(&line).map_err(|e| {
            CoreError::mcp(
                "mcp.deserialization_error",
                false,
                format!("Failed to deserialize MCP response: {e}"),
                Some(serde_json::json!({
                    "rawResponse": line,
                })),
            )
        })?;

        Ok(resp)
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
