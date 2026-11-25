use serde_json::json;
use starship_core::policy::TierPolicy;
use starship_core::safety::{RiskTier, SafetyArbiter, SecretScrubber};
use starship_db::audit::compute_audit_hash;
use starship_db::{Database, NewAuditEntry};
use starship_mcp::SecurityVerifier;
use std::path::{Path, PathBuf};

#[test]
fn host_tier_floor_cannot_be_lowered_by_an_app() {
    let policy = TierPolicy::default();
    assert_eq!(
        policy.calculate_effective_tier(Some(RiskTier::Safe), "eep.timeline.delete_clips"),
        RiskTier::Destructive
    );
    assert_eq!(
        policy.calculate_effective_tier(Some(RiskTier::Safe), "custom.some_action"),
        RiskTier::Mutating
    );
}

#[test]
fn approval_tokens_are_bound_to_arguments_and_single_use() {
    let arbiter = SafetyArbiter::new("security-suite-salt");
    let arguments = json!({ "clip": 4, "track": 1 });
    let request =
        arbiter.create_approval_request(7, 42, &arguments, RiskTier::Mutating, 1_700_000_000);

    arbiter
        .verify_and_consume_approval(7, 42, &arguments, &request.nonce, 1_700_000_001)
        .expect("first use succeeds");
    let replay = arbiter
        .verify_and_consume_approval(7, 42, &arguments, &request.nonce, 1_700_000_002)
        .expect_err("replay must fail");
    assert_eq!(replay.code, "safety.invalid_signature_token");

    let request =
        arbiter.create_approval_request(7, 43, &arguments, RiskTier::Mutating, 1_700_000_000);
    let mutated = arbiter
        .verify_and_consume_approval(7, 43, &json!({ "clip": 5 }), &request.nonce, 1_700_000_001)
        .expect_err("changed arguments must fail");
    assert_eq!(mutated.code, "safety.invalid_signature_token");
}

#[test]
fn manifest_pinning_rejects_tampered_binary() {
    let binary = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("security-suite-pinned-binary");
    std::fs::write(&binary, b"trusted binary").expect("write binary");
    let digest = SecurityVerifier::compute_binary_hash(&binary).expect("hash binary");
    SecurityVerifier::verify_pinned_hash(&binary, Some(&digest)).expect("trusted hash matches");

    std::fs::write(&binary, b"tampered binary").expect("tamper binary");
    let error =
        SecurityVerifier::verify_pinned_hash(&binary, Some(&digest)).expect_err("tamper detected");
    assert_eq!(error.code, "mcp.manifest_untrusted");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn peer_executable_mismatch_is_rejected() {
    let peer_pid = i32::try_from(std::process::id()).expect("pid fits i32");
    let error = SecurityVerifier::verify_peer_executable(peer_pid, Path::new("/usr/bin/false"))
        .expect_err("wrong executable must fail");
    assert_eq!(error.code, "mcp.so_peercred_failed");
}

#[test]
fn audit_chain_and_secret_scrubbing_hold() {
    let db = Database::open_in_memory().expect("open db");
    let canary = "sk-security-suite-canary";
    for action in ["security.setup", "security.call", "security.gate"] {
        db.record_audit_log(&NewAuditEntry {
            mission_id: None,
            app_id: "security-suite".to_owned(),
            action_type: action.to_owned(),
            risk_tier: 2,
            payload_json: json!({ "secret": canary, "action": action }).to_string(),
            status: "success".to_owned(),
            signature_token: None,
        })
        .expect("record audit");
    }
    assert_eq!(db.verify_audit_chain().expect("audit chain"), 3);

    let logs = db.list_audit_logs(10, 0).expect("audit logs");
    let scrubbed = SecretScrubber::scrub(&logs[0].payload_json, &[canary]);
    assert!(
        !scrubbed.contains(canary),
        "canary leaked into rendered output"
    );

    // Mutation proof: changing a single payload byte changes the chain hash.
    let original = compute_audit_hash(
        "0000000000000000000000000000000000000000000000000000000000000000",
        Some(1),
        "security-suite",
        "security.call",
        2,
        &json!({ "secret": canary }).to_string(),
        "success",
        None,
    );
    let mutated = compute_audit_hash(
        "0000000000000000000000000000000000000000000000000000000000000000",
        Some(1),
        "security-suite",
        "security.call",
        2,
        &json!({ "secret": "tampered" }).to_string(),
        "success",
        None,
    );
    assert_ne!(
        original, mutated,
        "audit hash must react to payload mutation"
    );
}

#[test]
fn webview_csp_and_renderer_sanitizer_are_present() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = std::fs::read_to_string(root.join("app/src-tauri/tauri.conf.json"))
        .expect("read tauri config");
    let parsed: serde_json::Value = serde_json::from_str(&config).expect("parse tauri config");
    let csp = parsed["app"]["security"]["csp"]
        .as_str()
        .expect("CSP configured");
    assert!(csp.contains("default-src 'self'"));
    assert!(csp.contains("script-src 'self'"));
    assert!(!csp.contains("unsafe-eval"));

    let sanitizer = std::fs::read_to_string(root.join("app/src/lib/sanitize.js"))
        .expect("read renderer sanitizer");
    assert!(sanitizer.contains("CHECK:javascript-scheme"));
    assert!(sanitizer.contains("blocked-scheme:"));
}
