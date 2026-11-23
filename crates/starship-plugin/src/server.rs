use crate::checkpoint::{
    TOOL_CHECKPOINT_CREATE, TOOL_CHECKPOINT_DROP, TOOL_CHECKPOINT_RESTORE,
    standard_checkpoint_tools,
};
use crate::progress::{ProgressNotification, ProgressReporter};
use crate::{AppManifest, BridgeServer, HelloResult, PluginTool, remove_manifest, write_manifest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::Arc;

pub type ToolFn = Box<
    dyn Fn(serde_json::Value, &ProgressReporter) -> Result<serde_json::Value, String>
        + Send
        + Sync
        + 'static,
>;

pub type CheckpointCreateFn = Box<dyn Fn(&str) -> Result<String, String> + Send + Sync + 'static>;
pub type CheckpointRestoreFn = Box<dyn Fn(&str) -> Result<(), String> + Send + Sync + 'static>;
pub type CheckpointDropFn = Box<dyn Fn(&str) -> Result<(), String> + Send + Sync + 'static>;

/// Checkpoint management callbacks implemented by a sub-application.
pub struct CheckpointCallbacks {
    pub create: CheckpointCreateFn,
    pub restore: CheckpointRestoreFn,
    pub drop: CheckpointDropFn,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: serde_json::Value, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// Turnkey Starship Plugin Bridge & MCP Server for sub-applications.
///
/// Handles automatic manifest publication, Unix domain socket serving,
/// stdio fallback for headless invocations (`--mcp`), tool dispatching,
/// progress notification streaming, and checkpoint contract lifecycle.
pub struct PluginBridge {
    manifest: AppManifest,
    tools: Vec<PluginTool>,
    handlers: HashMap<String, ToolFn>,
    checkpoint_callbacks: Option<CheckpointCallbacks>,
}

impl PluginBridge {
    /// Creates a new `PluginBridge` with the specified application manifest.
    #[must_use]
    pub fn new(manifest: AppManifest) -> Self {
        Self {
            manifest,
            tools: Vec::new(),
            handlers: HashMap::new(),
            checkpoint_callbacks: None,
        }
    }

    /// Registers a tool definition and its execution handler.
    pub fn register_tool<F>(&mut self, tool: PluginTool, handler: F)
    where
        F: Fn(serde_json::Value, &ProgressReporter) -> Result<serde_json::Value, String>
            + Send
            + Sync
            + 'static,
    {
        let name = tool.name.clone();
        self.tools.push(tool);
        self.handlers.insert(name, Box::new(handler));
    }

    /// Enables standard checkpoint contract tools (`starship/checkpoint/*`)
    /// and hooks up state snapshot / restore callbacks.
    pub fn enable_checkpoints(
        &mut self,
        create: impl Fn(&str) -> Result<String, String> + Send + Sync + 'static,
        restore: impl Fn(&str) -> Result<(), String> + Send + Sync + 'static,
        drop: impl Fn(&str) -> Result<(), String> + Send + Sync + 'static,
    ) {
        // Register checkpoint tool metadata
        for tool in standard_checkpoint_tools() {
            if !self.tools.iter().any(|t| t.name == tool.name) {
                self.tools.push(tool);
            }
        }

        self.checkpoint_callbacks = Some(CheckpointCallbacks {
            create: Box::new(create),
            restore: Box::new(restore),
            drop: Box::new(drop),
        });
    }

    /// Writes application manifest to `$XDG_DATA_HOME/starship/plugins/<app-id>.json`.
    pub fn publish_manifest(&self) -> io::Result<PathBuf> {
        write_manifest(&self.manifest)
    }

    /// Removes application manifest from `$XDG_DATA_HOME/starship/plugins/<app-id>.json`.
    pub fn unpublish_manifest(&self) -> io::Result<()> {
        remove_manifest(&self.manifest.app_id)
    }

    /// Processes a single line of JSON-RPC protocol message, returning the serialized response if applicable.
    pub fn handle_jsonrpc_line<F>(&self, line: &str, _progress_sink: F) -> Option<String>
    where
        F: FnMut(ProgressNotification),
    {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    -32700,
                    format!("Parse error: {e}"),
                );
                return serde_json::to_string(&err_resp).ok();
            }
        };

        let req_id = req.id.unwrap_or(serde_json::Value::Null);

        let response = match req.method.as_str() {
            // Starship Handshake Contract (§4.1)
            "starship/hello" => {
                let hello = HelloResult {
                    app_id: self.manifest.app_id.clone(),
                    name: self.manifest.name.clone(),
                    version: self.manifest.version.clone(),
                    tools: self.tools.clone(),
                };
                JsonRpcResponse::success(
                    req_id,
                    serde_json::to_value(hello).unwrap_or(serde_json::Value::Null),
                )
            }

            // Standard MCP Initialization
            "initialize" => {
                let info = serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": self.manifest.name,
                        "version": self.manifest.version
                    },
                    "capabilities": {
                        "tools": { "listChanged": false }
                    }
                });
                JsonRpcResponse::success(req_id, info)
            }

            // MCP Tool Enumeration
            "tools/list" => {
                let list_val = serde_json::json!({
                    "tools": self.tools
                });
                JsonRpcResponse::success(req_id, list_val)
            }

            // Ping
            "ping" => JsonRpcResponse::success(req_id, serde_json::json!({})),

            // Tool Invocation (§4.2)
            "tools/call" => {
                let params = req.params.unwrap_or_default();
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                let progress_token = params
                    .get("_meta")
                    .and_then(|m| m.get("progressToken"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("token_default")
                    .to_string();

                let reporter = ProgressReporter::new(serde_json::Value::String(progress_token));

                // Dispatch to tool handler
                if let Some(handler) = self.handlers.get(tool_name) {
                    match handler(args, &reporter) {
                        Ok(res) => JsonRpcResponse::success(req_id, res),
                        Err(err_msg) => JsonRpcResponse::error(req_id, -32000, err_msg),
                    }
                } else if tool_name == TOOL_CHECKPOINT_CREATE
                    || tool_name == "starship/checkpoint/create"
                {
                    let label = args
                        .get("label")
                        .and_then(|l| l.as_str())
                        .unwrap_or("snapshot");
                    if let Some(ref cb) = self.checkpoint_callbacks {
                        match (cb.create)(label) {
                            Ok(id) => JsonRpcResponse::success(
                                req_id,
                                serde_json::json!({ "checkpointId": id, "label": label }),
                            ),
                            Err(e) => JsonRpcResponse::error(req_id, -32001, e),
                        }
                    } else {
                        JsonRpcResponse::error(req_id, -32001, "Checkpoints not implemented")
                    }
                } else if tool_name == TOOL_CHECKPOINT_RESTORE
                    || tool_name == "starship/checkpoint/restore"
                {
                    let id = args
                        .get("checkpointId")
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    if let Some(ref cb) = self.checkpoint_callbacks {
                        match (cb.restore)(id) {
                            Ok(()) => JsonRpcResponse::success(
                                req_id,
                                serde_json::json!({ "status": "restored", "checkpointId": id }),
                            ),
                            Err(e) => JsonRpcResponse::error(req_id, -32001, e),
                        }
                    } else {
                        JsonRpcResponse::error(req_id, -32001, "Checkpoints not implemented")
                    }
                } else if tool_name == TOOL_CHECKPOINT_DROP
                    || tool_name == "starship/checkpoint/drop"
                {
                    let id = args
                        .get("checkpointId")
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    if let Some(ref cb) = self.checkpoint_callbacks {
                        match (cb.drop)(id) {
                            Ok(()) => JsonRpcResponse::success(
                                req_id,
                                serde_json::json!({ "status": "dropped", "checkpointId": id }),
                            ),
                            Err(e) => JsonRpcResponse::error(req_id, -32001, e),
                        }
                    } else {
                        JsonRpcResponse::error(req_id, -32001, "Checkpoints not implemented")
                    }
                } else {
                    JsonRpcResponse::error(req_id, -32601, format!("Unknown tool: '{tool_name}'"))
                }
            }

            // Task Cancellation (§4.3)
            "tools/cancel" => {
                JsonRpcResponse::success(req_id, serde_json::json!({ "cancelled": true }))
            }

            unknown => {
                JsonRpcResponse::error(req_id, -32601, format!("Method not found: '{unknown}'"))
            }
        };

        serde_json::to_string(&response).ok()
    }

    /// Serves incoming JSON-RPC protocol over any reader/writer pair (e.g. socket or stdio).
    pub fn serve_stream<R: BufRead, W: Write>(
        &self,
        mut reader: R,
        mut writer: W,
    ) -> io::Result<()> {
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            let writer_mutex = Arc::new(std::sync::Mutex::new(&mut writer));
            let writer_clone = writer_mutex.clone();

            let progress_sink = move |notif: ProgressNotification| {
                if let Ok(json) = serde_json::to_string(&notif)
                    && let Ok(mut w) = writer_clone.lock()
                {
                    let _ = writeln!(w, "{json}");
                    let _ = w.flush();
                }
            };

            if let Some(resp_json) = self.handle_jsonrpc_line(&line, progress_sink)
                && let Ok(mut w) = writer_mutex.lock()
            {
                writeln!(w, "{resp_json}")?;
                w.flush()?;
            }
            line.clear();
        }
        Ok(())
    }

    /// Serves JSON-RPC protocol over standard I/O (stdin/stdout) for headless child processes (`--mcp`).
    pub fn serve_stdio(&self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        self.serve_stream(stdin.lock(), stdout.lock())
    }

    /// Binds the Unix domain socket server and publishes the application manifest.
    pub fn serve_socket(&self) -> io::Result<BridgeServer> {
        let _ = self.publish_manifest();
        BridgeServer::bind(&self.manifest.app_id)
    }

    /// Processes an accepted client socket connection.
    pub fn handle_client(&self, stream: UnixStream) -> io::Result<()> {
        let stream_clone = stream.try_clone()?;
        let reader = BufReader::new(stream);
        self.serve_stream(reader, stream_clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ResourceProfile;
    use std::io::Cursor;

    fn make_test_manifest() -> AppManifest {
        AppManifest {
            schema_version: "1.0".to_string(),
            app_id: "testapp".to_string(),
            name: "Test App".to_string(),
            version: "0.1.0".to_string(),
            binary_path: "/usr/bin/testapp".to_string(),
            launch_args: vec![],
            mcp_cli_args: vec!["--mcp".to_string()],
            socket_path: None,
            capabilities: serde_json::json!({ "featureX": true }),
        }
    }

    #[test]
    fn test_plugin_bridge_handshake_and_tool_call() {
        let mut bridge = PluginBridge::new(make_test_manifest());

        // Register custom tool
        bridge.register_tool(
            PluginTool {
                name: "test.echo".to_string(),
                description: "Echoes input text".to_string(),
                risk_tier: 1,
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": { "text": { "type": "string" } },
                    "required": ["text"]
                }),
                resource_profile: Some(ResourceProfile::light(64)),
            },
            |args, _progress| {
                let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                Ok(serde_json::json!({ "echo": text }))
            },
        );

        // 1. Test starship/hello
        let hello_req = r#"{"jsonrpc":"2.0","id":1,"method":"starship/hello"}"#;
        let hello_resp_str = bridge
            .handle_jsonrpc_line(hello_req, |_| {})
            .expect("response");
        let hello_resp: serde_json::Value = serde_json::from_str(&hello_resp_str).expect("json");
        assert_eq!(hello_resp["result"]["appId"], "testapp");
        assert_eq!(hello_resp["result"]["tools"].as_array().unwrap().len(), 1);

        // 2. Test tools/call
        let call_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"test.echo","arguments":{"text":"hello starship"}}}"#;
        let call_resp_str = bridge
            .handle_jsonrpc_line(call_req, |_| {})
            .expect("response");
        let call_resp: serde_json::Value = serde_json::from_str(&call_resp_str).expect("json");
        assert_eq!(call_resp["result"]["echo"], "hello starship");
    }

    #[test]
    fn test_plugin_bridge_checkpoints_lifecycle() {
        let mut bridge = PluginBridge::new(make_test_manifest());

        bridge.enable_checkpoints(
            |label| Ok(format!("ckpt_{label}_123")),
            |id| {
                if id == "ckpt_valid_123" {
                    Ok(())
                } else {
                    Err("not found".into())
                }
            },
            |_id| Ok(()),
        );

        // Create checkpoint
        let create_req = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"starship/checkpoint/create","arguments":{"label":"pre_cut"}}}"#;
        let create_resp_str = bridge
            .handle_jsonrpc_line(create_req, |_| {})
            .expect("response");
        let create_resp: serde_json::Value = serde_json::from_str(&create_resp_str).expect("json");
        assert_eq!(create_resp["result"]["checkpointId"], "ckpt_pre_cut_123");

        // Restore checkpoint
        let restore_req = r#"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"starship/checkpoint/restore","arguments":{"checkpointId":"ckpt_valid_123"}}}"#;
        let restore_resp_str = bridge
            .handle_jsonrpc_line(restore_req, |_| {})
            .expect("response");
        let restore_resp: serde_json::Value =
            serde_json::from_str(&restore_resp_str).expect("json");
        assert_eq!(restore_resp["result"]["status"], "restored");
    }

    #[test]
    fn test_serve_stream_piped_io() {
        let mut bridge = PluginBridge::new(make_test_manifest());
        bridge.register_tool(
            PluginTool {
                name: "test.ping".into(),
                description: "ping".into(),
                risk_tier: 1,
                parameters: serde_json::json!({}),
                resource_profile: None,
            },
            |_args, _progress| Ok(serde_json::json!({ "pong": true })),
        );

        let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"test.ping\",\"arguments\":{}}}\n";
        let reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();

        bridge
            .serve_stream(reader, &mut output)
            .expect("serve stream");
        let out_str = String::from_utf8(output).expect("valid utf8");
        assert!(out_str.contains("\"pong\":true"));
    }
}
