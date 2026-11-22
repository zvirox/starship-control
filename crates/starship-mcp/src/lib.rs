pub mod client;
pub mod registry;

pub use client::{McpRequest, McpResponse, UnixTransportClient};
pub use registry::{
    get_known_ecosystem_apps, scan_and_sync_manifests, seed_default_manifests_if_needed,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;

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
}
