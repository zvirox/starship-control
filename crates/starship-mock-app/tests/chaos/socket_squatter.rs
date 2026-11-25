use super::harness::{ChaosMission, now_ms};
use starship_core::MissionState;
use starship_core::executor::{
    CrashRecoveryManager, InterruptedStepInfo, RecoveryAction, StepState,
};
use starship_core::safety::RiskTier;
use starship_mcp::{SecurityVerifier, UnixTransportClient};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::thread;

const WRONG_BINARY: &str = "/usr/bin/false";

#[test]
fn peer_executable_mismatch_is_rejected_as_safety_failure() {
    let chaos = ChaosMission::new();
    chaos.start_step();

    let peer_pid = i32::try_from(std::process::id()).expect("pid fits i32");
    let error = SecurityVerifier::verify_peer_executable(peer_pid, WRONG_BINARY.as_ref())
        .expect_err("wrong pinned executable must be rejected");
    assert_eq!(error.code, "mcp.so_peercred_failed");

    chaos.fail_step(&error.code);
    chaos.assert_final(MissionState::Failed, Some(&error.code));
}

#[tokio::test]
async fn same_uid_squatter_is_observed_and_drives_mission_failure() {
    let chaos = ChaosMission::new();
    chaos.start_step();

    let socket = PathBuf::from(format!(
        "/tmp/starship-chaos-squatter-{}.sock",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).expect("bind squatter socket");
    let squatter = thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            use std::io::Write;
            let _ = stream.write_all(b"not-json\n");
        }
    });

    let connected = UnixTransportClient::connect(&socket).await;
    let error = match connected {
        Ok(mut client) => client
            .send_request("ping", None)
            .await
            .expect_err("squatter never sends valid JSON-RPC"),
        Err(error) => error,
    };
    assert!(
        error.code.starts_with("mcp."),
        "unexpected code: {}",
        error.code
    );
    squatter.join().expect("squatter thread");
    let _ = std::fs::remove_file(&socket);

    chaos.fail_step(&error.code);
    chaos.assert_final(MissionState::Failed, Some(&error.code));

    // Documented gap: SO_PEERCRED alone cannot reject a same-UID squatter.
    // The peer-executable check above must be wired in by the caller.
    let interrupted = InterruptedStepInfo {
        step_id: chaos.step_id,
        step_index: 0,
        app_id: "eep".to_owned(),
        tool_name: "eep.timeline.cut_interval".to_owned(),
        risk_tier: RiskTier::Mutating,
        checkpoint_id: None,
    };
    let (state, approval) = CrashRecoveryManager::resolve_step_recovery(
        &interrupted,
        RecoveryAction::Resume,
        &chaos.arbiter,
        chaos.plan_id,
        now_ms(),
    )
    .expect("resolve recovery");
    assert_eq!(state, StepState::AwaitingApproval);
    assert!(approval.is_some(), "tier 2 resume requires fresh approval");
}
