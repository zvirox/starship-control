use super::harness::ChaosMission;
use starship_core::MissionState;
use starship_mcp::SecurityVerifier;
use std::path::PathBuf;

fn pinned_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("chaos-pinned-binary")
}

#[test]
fn tampered_manifest_binary_fails_pinned_hash_verification() {
    let chaos = ChaosMission::new();
    chaos.start_step();

    let binary = pinned_binary();
    std::fs::write(&binary, b"trusted binary contents").expect("write pinned binary");
    let digest = SecurityVerifier::compute_binary_hash(&binary).expect("hash binary");
    SecurityVerifier::verify_pinned_hash(&binary, Some(&digest)).expect("trusted hash matches");

    std::fs::write(&binary, b"tampered binary contents").expect("tamper binary");
    let error =
        SecurityVerifier::verify_pinned_hash(&binary, Some(&digest)).expect_err("tamper detected");
    assert_eq!(error.code, "mcp.manifest_untrusted");

    chaos.fail_step(&error.code);
    chaos.assert_final(MissionState::Failed, Some(&error.code));
    let _ = std::fs::remove_file(binary);
}
