pub mod arbiter;
pub mod secrets;

pub use arbiter::{ApprovalRequest, ExecutionPermission, RiskTier, SafetyArbiter};
pub use secrets::{SecretScrubber, SecretVault};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tier_1_safe_tool_autonomous_execution() {
        let arbiter = SafetyArbiter::default();

        // Tier 1 tool classification
        assert_eq!(arbiter.classify_tool("timeline.inspect"), RiskTier::Safe);
        assert_eq!(arbiter.classify_tool("network.status"), RiskTier::Safe);
        assert_eq!(
            arbiter.classify_tool("disk.query_free_space"),
            RiskTier::Safe
        );

        // Tier 1 executes autonomously without any signature token
        let permission = arbiter.evaluate_step(1, 101, "timeline.inspect", RiskTier::Safe, None);
        assert_eq!(permission, Ok(ExecutionPermission::Autonomous));
    }

    #[test]
    fn test_tier_2_mutating_tool_blocked_without_approval() {
        // Verification requirement for 5.2:
        // "An unapproved Tier 2 mutation is blocked at the Rust core level, returning safety.permission_denied."
        let arbiter = SafetyArbiter::default();

        assert_eq!(
            arbiter.classify_tool("timeline.cut_interval"),
            RiskTier::Mutating
        );
        assert_eq!(arbiter.classify_tool("audio.cleanup"), RiskTier::Mutating);

        let result =
            arbiter.evaluate_step(42, 101, "timeline.cut_interval", RiskTier::Mutating, None);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.code, "safety.permission_denied");
        assert!(
            err.user_message
                .contains("requires explicit human approval token")
        );
    }

    #[test]
    fn test_tier_2_approved_with_valid_token() {
        let arbiter = SafetyArbiter::default();
        let plan_id = 42;
        let step_id = 101;
        let tool_name = "timeline.cut_interval";

        let token = arbiter.generate_signature_token(plan_id, step_id, tool_name);
        assert!(token.starts_with("usr_sig_"));

        let permission = arbiter.evaluate_step(
            plan_id,
            step_id,
            tool_name,
            RiskTier::Mutating,
            Some(&token),
        );

        assert_eq!(
            permission,
            Ok(ExecutionPermission::Approved {
                signature_token: token
            })
        );
    }

    #[test]
    fn test_tampered_signature_token_fails() {
        let arbiter = SafetyArbiter::default();
        let plan_id = 42;
        let step_id = 101;
        let tool_name = "timeline.cut_interval";

        let fake_token = "usr_sig_bad00000000000000000000000000000";
        let result = arbiter.evaluate_step(
            plan_id,
            step_id,
            tool_name,
            RiskTier::Mutating,
            Some(fake_token),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "safety.invalid_signature_token");
    }

    #[test]
    fn test_tier_3_destructive_tool_classification_and_gating() {
        let arbiter = SafetyArbiter::default();

        assert_eq!(
            arbiter.classify_tool("cache.purge_all"),
            RiskTier::Destructive
        );
        assert_eq!(
            arbiter.classify_tool("take.delete_file"),
            RiskTier::Destructive
        );

        // Blocked without token
        let res = arbiter.evaluate_step(10, 201, "cache.purge_all", RiskTier::Destructive, None);
        assert_eq!(res.unwrap_err().code, "safety.permission_denied");

        // Allowed with valid token
        let token = arbiter.generate_signature_token(10, 201, "cache.purge_all");
        let res = arbiter.evaluate_step(
            10,
            201,
            "cache.purge_all",
            RiskTier::Destructive,
            Some(&token),
        );
        assert!(res.is_ok());
    }

    // =========================================================================
    // GAP-CLOSURE-PLAN.md §10.3: Argument-Bound Approvals Test Suite
    // Tests: replay, expired, args mutated after approval, wrong step.
    // =========================================================================

    #[test]
    fn test_argument_bound_approval_valid() {
        let arbiter = SafetyArbiter::default();
        let args = json!({ "path": "/home/user/project/take1.raw", "force": true });
        let now = 1_700_000_000u64;

        let req = arbiter.create_approval_request(100, 1, &args, RiskTier::Mutating, now);
        assert_eq!(req.plan_id, 100);
        assert_eq!(req.step_id, 1);
        assert_eq!(req.expires_at, now + 300);

        // Verification at dispatch time (within TTL, identical args)
        let dispatch_result =
            arbiter.verify_and_consume_approval(100, 1, &args, &req.nonce, now + 10);
        assert!(dispatch_result.is_ok(), "Valid approval must succeed");
    }

    #[test]
    fn test_argument_bound_approval_replay_rejected() {
        let arbiter = SafetyArbiter::default();
        let args = json!({ "action": "trim", "duration": 500 });
        let now = 1_700_000_000u64;

        let req = arbiter.create_approval_request(100, 2, &args, RiskTier::Mutating, now);

        // First execution succeeds
        let first = arbiter.verify_and_consume_approval(100, 2, &args, &req.nonce, now + 5);
        assert!(first.is_ok());

        // Replay attempt byte-identical must be rejected
        let replay = arbiter.verify_and_consume_approval(100, 2, &args, &req.nonce, now + 10);
        assert!(replay.is_err(), "Replay attack must fail");
        let err = replay.unwrap_err();
        assert_eq!(err.code, "safety.invalid_signature_token");
        assert!(err.user_message.contains("replay prevented"));
    }

    #[test]
    fn test_argument_bound_approval_expired_rejected() {
        let arbiter = SafetyArbiter::default();
        let args = json!({ "purge": "all" });
        let now = 1_700_000_000u64;

        let req = arbiter.create_approval_request(100, 3, &args, RiskTier::Destructive, now);

        // TTL is 300s (5 minutes). Dispatch at now + 301s must be rejected.
        let expired = arbiter.verify_and_consume_approval(100, 3, &args, &req.nonce, now + 301);
        assert!(expired.is_err(), "Expired approval must fail");
        let err = expired.unwrap_err();
        assert_eq!(err.code, "safety.invalid_signature_token");
        assert!(err.user_message.contains("expired"));
    }

    #[test]
    fn test_argument_bound_approval_args_mutated_rejected() {
        let arbiter = SafetyArbiter::default();
        let approved_args = json!({ "path": "/tmp/harmless.txt" });
        let now = 1_700_000_000u64;

        let req =
            arbiter.create_approval_request(100, 4, &approved_args, RiskTier::Destructive, now);

        // Malicious or buggy mutation of arguments after user signed approval
        let mutated_args = json!({ "path": "/etc/shadow" });
        let dispatch =
            arbiter.verify_and_consume_approval(100, 4, &mutated_args, &req.nonce, now + 10);

        assert!(dispatch.is_err(), "Mutated arguments must fail");
        let err = dispatch.unwrap_err();
        assert_eq!(err.code, "safety.invalid_signature_token");
        assert!(err.user_message.contains("mutated after approval"));
    }

    #[test]
    fn test_argument_bound_approval_wrong_step_rejected() {
        let arbiter = SafetyArbiter::default();
        let args = json!({ "volume": 0.8 });
        let now = 1_700_000_000u64;

        // Approved for plan 100, step 5
        let req = arbiter.create_approval_request(100, 5, &args, RiskTier::Mutating, now);

        // Attacker attempts to apply token to step 6
        let dispatch = arbiter.verify_and_consume_approval(100, 6, &args, &req.nonce, now + 10);

        assert!(dispatch.is_err(), "Token applied to wrong step must fail");
        let err = dispatch.unwrap_err();
        assert_eq!(err.code, "safety.invalid_signature_token");
        assert!(err.user_message.contains("does not match"));
    }
}
