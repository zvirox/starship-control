pub mod arbiter;

pub use arbiter::{ExecutionPermission, RiskTier, SafetyArbiter};

#[cfg(test)]
mod tests {
    use super::*;

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
}
