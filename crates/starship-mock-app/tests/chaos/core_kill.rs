use super::harness::{ChaosMission, assert_process_gone, now_ms, scenario_path};
use serde_json::json;
use starship_core::MissionState;
use starship_core::executor::{
    CrashRecoveryManager, InterruptedStepInfo, PauseReason, RecoveryAction, StepState,
};
use starship_core::safety::RiskTier;
use starship_mcp::{ContainmentConfig, McpSession, StdioTransportClient};

fn kill_process(pid: u32) {
    let status = std::process::Command::new("kill")
        .arg("-9")
        .arg(pid.to_string())
        .status()
        .expect("invoke kill");
    assert!(status.success(), "kill -9 failed for process {pid}");
}

#[tokio::test]
async fn core_kill_mid_mission_requires_fresh_approval_on_resume() {
    let chaos = ChaosMission::new();
    chaos.start_step();

    let scenario = scenario_path("hang").to_string_lossy().into_owned();
    let client = StdioTransportClient::spawn_with_containment(
        env!("CARGO_BIN_EXE_starship-mock-app"),
        &["--mcp", "--scenario", scenario.as_str()],
        &ContainmentConfig {
            enabled: false,
            ..ContainmentConfig::default()
        },
    )
    .expect("spawn hang app without containment");
    let pid = client.pid().expect("child pid");
    let mut session = McpSession::new(Box::new(client));
    session.handshake().await.expect("handshake");
    {
        let pending = session.call_tool("mock.inspect", json!({}), 300);
        tokio::pin!(pending);
        let _ = tokio::time::timeout(std::time::Duration::from_millis(350), &mut pending).await;
    }

    kill_process(pid);
    drop(session);

    chaos.interrupt_step();
    let interrupted = InterruptedStepInfo {
        step_id: chaos.step_id,
        step_index: 0,
        app_id: "eep".to_owned(),
        tool_name: "eep.timeline.cut_interval".to_owned(),
        risk_tier: RiskTier::Mutating,
        checkpoint_id: None,
    };
    let (step_state, approval) = CrashRecoveryManager::resolve_step_recovery(
        &interrupted,
        RecoveryAction::Resume,
        &chaos.arbiter,
        chaos.plan_id,
        now_ms(),
    )
    .expect("resolve crash recovery");
    assert_eq!(step_state, StepState::AwaitingApproval);
    assert!(
        approval.is_some(),
        "tier 2 must never auto-resume after kill -9"
    );

    chaos.pause(PauseReason::Approval);
    chaos.assert_final(
        MissionState::Paused(PauseReason::Approval),
        Some("chaos.kill9"),
    );
    assert_process_gone(pid);
}
