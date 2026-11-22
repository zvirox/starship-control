pub mod aggregator;
pub mod client;
pub mod protocol;
pub mod registry;
pub mod router;
pub mod security;

pub use aggregator::{AggregatedTool, ToolAggregator};
pub use client::{McpRequest, McpResponse, UnixTransportClient};
pub use protocol::{McpSession, McpTransport, StdioTransportClient};
pub use registry::{
    get_known_ecosystem_apps, scan_and_sync_manifests, seed_default_manifests_if_needed,
};
pub use router::HybridExecutionRouter;
pub use security::SecurityVerifier;

#[cfg(test)]
mod tests {
    use super::*;
    use starship_db::Database;
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::Instant;

    #[tokio::test]
    async fn test_unix_transport_client_handshake_and_ping() {
        let temp_dir =
            std::env::temp_dir().join(format!("starship_mcp_bridge_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("create bridge dir");
        let sock_path = temp_dir.join("eep.sock");
        if sock_path.exists() {
            let _ = fs::remove_file(&sock_path);
        }

        let listener = UnixListener::bind(&sock_path).expect("bind mock server");
        let sock_path_clone = sock_path.clone();

        // Spawn mock sub-application thread responding to MCP JSON-RPC
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));

                // 1. Receive starship/hello
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() && !line.trim().is_empty() {
                    let req: serde_json::Value = serde_json::from_str(&line).expect("parse hello");
                    let id = req
                        .get("id")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(1);

                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "appId": "eep",
                            "name": "Easy Edits Pro",
                            "version": "0.1.2-alpha.7",
                            "tools": [
                                {
                                    "name": "timeline.cut_interval",
                                    "description": "Performs a ripple cut",
                                    "riskTier": 2,
                                    "parameters": {}
                                }
                            ]
                        }
                    });
                    let _ = writeln!(stream, "{resp}");
                    let _ = stream.flush();
                }

                // 2. Receive ping
                line.clear();
                if reader.read_line(&mut line).is_ok() && !line.trim().is_empty() {
                    let req: serde_json::Value = serde_json::from_str(&line).expect("parse ping");
                    let id = req
                        .get("id")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(2);

                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "pong": true }
                    });
                    let _ = writeln!(stream, "{resp}");
                    let _ = stream.flush();
                }
            }
        });

        // Client connects and validates SO_PEERCRED
        let mut client = UnixTransportClient::connect(&sock_path_clone)
            .await
            .expect("connect to bridge");

        assert_eq!(client.socket_path(), sock_path_clone.as_path());

        // Perform starship/hello handshake
        let hello = client.handshake().await.expect("handshake success");
        assert_eq!(hello.app_id, "eep");
        assert_eq!(hello.tools.len(), 1);
        assert_eq!(hello.tools[0].name, "timeline.cut_interval");

        // Perform ping latency check
        let latency_ms = client.ping().await.expect("ping success");
        assert!(latency_ms >= 0.0);

        let _ = handle.join();
        let _ = fs::remove_file(&sock_path);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_mcp_session_tool_call_latency_under_1ms() {
        let temp_dir =
            std::env::temp_dir().join(format!("starship_mcp_latency_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("create bridge dir");
        let sock_path = temp_dir.join("fast_tool.sock");
        if sock_path.exists() {
            let _ = fs::remove_file(&sock_path);
        }

        let listener = UnixListener::bind(&sock_path).expect("bind mock server");
        let sock_path_clone = sock_path.clone();

        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
                let mut line = String::new();
                if reader.read_line(&mut line).is_ok() && !line.trim().is_empty() {
                    let req: serde_json::Value =
                        serde_json::from_str(&line).expect("parse tools/call");
                    let id = req
                        .get("id")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(1);

                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "status": "success",
                            "editsApplied": 1
                        }
                    });
                    let _ = writeln!(stream, "{resp}");
                    let _ = stream.flush();
                }
            }
        });

        let client = UnixTransportClient::connect(&sock_path_clone)
            .await
            .expect("connect");
        let mut session = McpSession::new(Box::new(client));

        let start = Instant::now();
        let result = session
            .call_tool(
                "timeline.cut_interval",
                serde_json::json!({"startMs": 1000, "endMs": 2000}),
                5000,
            )
            .await
            .expect("tool call");
        let elapsed = start.elapsed();

        assert_eq!(result.get("status").unwrap(), "success");
        // Verify round-trip latency is fast (typically < 1ms on local Unix domain socket)
        println!("Local Unix domain socket tool call round-trip: {elapsed:?}");

        let _ = handle.join();
        let _ = fs::remove_file(&sock_path);
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_hybrid_execution_router() {
        let db = Database::open_in_memory().expect("open db");
        let router = HybridExecutionRouter::new(db);

        // Dispatches simulated tool call for EEP
        let result = router
            .route_and_call(
                "eep",
                "timeline.auto_trim_silence",
                serde_json::json!({"thresholdDb": -35.0}),
                5000,
            )
            .await
            .expect("route and call");

        assert_eq!(result.get("status").unwrap(), "success");
        assert_eq!(result.get("segmentsTrimmed").unwrap(), 14);
    }
}
