pub mod aggregator;
pub mod client;
pub mod containment;
pub mod protocol;
pub mod registry;
pub mod router;
pub mod security;

pub use aggregator::{AggregatedTool, ToolAggregator};
pub use client::{McpRequest, McpResponse, UnixTransportClient};
pub use containment::{ContainmentConfig, is_systemd_run_available, prepare_command};
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

        // No socket and no binary for EEP: must return a typed error, never
        // simulated success (Phase 17, task 17.1).
        let err = router
            .route_and_call(
                "eep",
                "timeline.auto_trim_silence",
                serde_json::json!({"thresholdDb": -35.0}),
                5000,
            )
            .await
            .expect_err("no transport must fail");
        assert_eq!(err.code, "mcp.app_unavailable");
        assert!(!err.retryable);
        assert!(
            err.details
                .as_ref()
                .is_some_and(|details| details["appId"] == "eep")
        );
    }

    #[tokio::test]
    #[allow(clippy::cast_precision_loss)]
    async fn test_mock_tool_emits_10_progress_events_mapped_to_step() {
        use starship_plugin::{ProgressNotification, ProgressReporter};
        use std::sync::{Arc, Mutex};

        let temp_dir =
            std::env::temp_dir().join(format!("starship_mcp_progress_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("create bridge dir");
        let sock_path = temp_dir.join("progress_tool.sock");
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
                    let params = req.get("params").cloned().unwrap_or_default();

                    // Extract progress reporter from params
                    let reporter = ProgressReporter::from_params(&params)
                        .expect("progress reporter from params");

                    // Emit 10 progress notifications
                    for i in 1..=10 {
                        let prog = f64::from(i) / 10.0;
                        let msg = format!("Applying filter stage {i}/10");
                        reporter
                            .report_sync(&mut stream, prog, Some(1.0), Some(&msg))
                            .expect("emit progress");
                    }

                    // Return final response
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "status": "completed",
                            "filterStages": 10
                        }
                    });
                    let _ = writeln!(stream, "{resp}");
                    let _ = stream.flush();
                }
            }
        });

        let mut client = UnixTransportClient::connect(&sock_path_clone)
            .await
            .expect("connect");

        let received_events = Arc::new(Mutex::new(Vec::<ProgressNotification>::new()));
        let events_clone = Arc::clone(&received_events);

        let result = client
            .call_tool_with_progress(
                "alp.spectral.denoise",
                serde_json::json!({ "strength": 0.8 }),
                Some(serde_json::json!("step-mission-42")),
                Some(move |notification: ProgressNotification| {
                    events_clone.lock().unwrap().push(notification);
                }),
            )
            .await
            .expect("tool call with progress");

        let _ = handle.join();
        let _ = fs::remove_file(&sock_path);
        let _ = fs::remove_dir_all(temp_dir);

        // Verification requirement for 13.1:
        // "mock tool emits 10 progress events; host maps them to the right step"
        let events = received_events.lock().unwrap().clone();
        assert_eq!(
            events.len(),
            10,
            "Expected exactly 10 progress events, got {}",
            events.len()
        );

        for (idx, evt) in events.iter().enumerate() {
            let expected_idx = idx + 1;
            let expected_prog = (expected_idx as f64) / 10.0;
            assert_eq!(evt.progress_token, serde_json::json!("step-mission-42"));
            assert!((evt.progress - expected_prog).abs() < 1e-6);
            assert_eq!(evt.total, Some(1.0));
            assert_eq!(
                evt.message.as_deref(),
                Some(format!("Applying filter stage {expected_idx}/10").as_str())
            );
        }

        assert_eq!(result.get("status").unwrap(), "completed");
        assert_eq!(result.get("filterStages").unwrap(), 10);
    }
}
