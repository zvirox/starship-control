use super::harness::{ChaosMission, assert_process_gone, scenario_path};
use serde_json::json;
use starship_core::MissionState;
use starship_core::executor::{FailureClass, RetryPolicy, classify_error};
use starship_core::safety::RiskTier;
use starship_mcp::{ContainmentConfig, McpSession, StdioTransportClient};

fn direct_spawn(scenario: &str) -> StdioTransportClient {
    StdioTransportClient::spawn_with_containment(
        env!("CARGO_BIN_EXE_starship-mock-app"),
        &["--mcp", "--scenario", scenario],
        &ContainmentConfig {
            enabled: false,
            ..ContainmentConfig::default()
        },
    )
    .expect("spawn mock app without containment")
}

#[tokio::test]
async fn app_killed_mid_step_drives_retryable_failure() {
    let chaos = ChaosMission::new();
    chaos.start_step();

    let scenario = scenario_path("crash").to_string_lossy().into_owned();
    let client = direct_spawn(scenario.as_str());
    let pid = client.pid().expect("child pid");
    let mut session = McpSession::new(Box::new(client));
    session.handshake().await.expect("handshake before crash");

    let error = session
        .call_tool("mock.inspect", json!({}), 2_000)
        .await
        .expect_err("crashed app cannot complete tool call");
    assert!(
        error.code == "mcp.socket_unreachable" || error.code == "mcp.execution_failed",
        "unexpected error code: {}",
        error.code
    );
    assert!(error.retryable, "crashed tool call must be retryable");

    let failure_class = classify_error(&error.code);
    assert_eq!(failure_class, FailureClass::Transient);
    let retry = RetryPolicy::default().evaluate_retry(RiskTier::Safe, failure_class, true, true, 1);
    assert!(retry.should_retry, "transient app death should be retried");

    chaos.fail_step(&error.code);
    chaos.assert_final(MissionState::Failed, Some(&error.code));
    assert_process_gone(pid);
}
