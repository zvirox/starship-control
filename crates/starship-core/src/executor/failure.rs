use serde::{Deserialize, Serialize};

/// High-level failure taxonomy classes (GAP-CLOSURE-PLAN.md §2.1 & §11.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    Transient,
    AppFailure,
    Resource,
    ModelOutput,
    Safety,
    Environment,
    User,
}

impl FailureClass {
    #[must_use]
    pub fn allows_retry(&self) -> bool {
        matches!(self, Self::Transient | Self::AppFailure | Self::Environment)
    }

    #[must_use]
    pub fn requires_user_notification(&self) -> bool {
        matches!(
            self,
            Self::Safety | Self::AppFailure | Self::Resource | Self::Environment
        )
    }
}

/// Classifies a canonical error code into its corresponding `FailureClass` (GAP-CLOSURE-PLAN.md §11.6).
#[must_use]
pub fn classify_error(code: &str) -> FailureClass {
    match code {
        // Transient: network timeouts, rate limits, temporary socket connection drops
        "mcp.socket_unreachable"
        | "mcp.handshake_timeout"
        | "infer.cloud_rate_limited"
        | "infer.cloud_endpoint_unreachable"
        | "infer.download_failed" => FailureClass::Transient,

        // App failure: sub-app crashes, internal execution errors, lease expirations
        "mcp.execution_failed"
        | "mcp.app_spawn_failed"
        | "mcp.lease_expired"
        | "mcp.resource_locked"
        | "mcp.tool_not_found" => FailureClass::AppFailure,

        // Resource: local VRAM/OOM, disk full, governor throttling
        "infer.model_out_of_memory" | "disk.no_space" => FailureClass::Resource,

        // Model output: context length exceeded, invalid plan json, checksum mismatch
        "infer.context_length_exceeded"
        | "infer.model_not_found"
        | "infer.checksum_mismatch"
        | "ipc.cycle_detected"
        | "plan.invalid_schema"
        | "plan.cycle_detected"
        | "plan.unknown_tool"
        | "plan.unknown_app"
        | "plan.args_invalid"
        | "plan.duplicate_step_index"
        | "plan.self_dependency"
        | "plan.missing_dependency"
        | "plan.max_steps_exceeded"
        | "plan.max_tier3_exceeded"
        | "plan.invalid_risk_tier"
        | "plan.empty_plan"
        | "plan.empty_goal"
        | "planner.unsupported_version"
        | "pipeline.telemetry_required" => FailureClass::ModelOutput,

        // Safety: approval rejections, permission violations, tamper, manifest untrusted
        "safety.approval_rejected"
        | "safety.permission_denied"
        | "safety.invalid_signature_token"
        | "safety.tier_violation"
        | "mcp.manifest_untrusted"
        | "mcp.so_peercred_failed"
        | "db.audit_tampered" => FailureClass::Safety,

        // Environment: DB read/write failure, migrations, missing windows/portals
        "db.read_failed"
        | "db.write_failed"
        | "db.constraint_violation"
        | "db.migration_failed"
        | "ipc.window_not_found"
        | "mcp.app_unavailable" => FailureClass::Environment,

        // User / Bad State: user abort or illegal state transition
        "ipc.bad_state" | "ipc.invalid_payload" => FailureClass::User,

        // Fallback domain prefixes
        c if c.starts_with("safety.") => FailureClass::Safety,
        c if c.starts_with("db.") => FailureClass::Environment,
        c if c.starts_with("infer.") => FailureClass::Transient,
        _ => FailureClass::AppFailure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_api_md_code_maps_to_exactly_one_failure_class() {
        // Enumeration of all canonical error codes declared in API.md §1.2
        let api_codes = [
            // infer.*
            "infer.model_not_found",
            "infer.model_out_of_memory",
            "infer.download_failed",
            "infer.checksum_mismatch",
            "infer.cloud_auth_failed",
            "infer.cloud_rate_limited",
            "infer.cloud_endpoint_unreachable",
            "infer.context_length_exceeded",
            // mcp.*
            "mcp.socket_unreachable",
            "mcp.so_peercred_failed",
            "mcp.handshake_timeout",
            "mcp.tool_not_found",
            "mcp.execution_failed",
            "mcp.app_spawn_failed",
            "mcp.app_unavailable",
            "mcp.lease_expired",
            // safety.*
            "safety.approval_rejected",
            "safety.permission_denied",
            "safety.invalid_signature_token",
            "safety.tier_violation",
            // db.*
            "db.read_failed",
            "db.write_failed",
            "db.constraint_violation",
            "db.migration_failed",
            // ipc.*
            "ipc.invalid_payload",
            "ipc.bad_state",
            "ipc.window_not_found",
            "ipc.cycle_detected",
            // plan.*
            "plan.invalid_schema",
            "plan.cycle_detected",
            "plan.unknown_tool",
            "plan.unknown_app",
            "plan.args_invalid",
            "plan.duplicate_step_index",
            "plan.self_dependency",
            "plan.missing_dependency",
            "plan.max_steps_exceeded",
            "plan.max_tier3_exceeded",
            "plan.invalid_risk_tier",
            "plan.empty_plan",
            "plan.empty_goal",
            "planner.unsupported_version",
            "pipeline.telemetry_required",
        ];

        for code in api_codes {
            let class = classify_error(code);
            // Verify class is deterministically mapped
            match code {
                "mcp.socket_unreachable" | "mcp.handshake_timeout" | "infer.cloud_rate_limited" => {
                    assert_eq!(class, FailureClass::Transient);
                }
                "mcp.execution_failed" | "mcp.app_spawn_failed" | "mcp.lease_expired" => {
                    assert_eq!(class, FailureClass::AppFailure);
                }
                "infer.model_out_of_memory" => {
                    assert_eq!(class, FailureClass::Resource);
                }
                "infer.context_length_exceeded"
                | "infer.checksum_mismatch"
                | "plan.invalid_schema"
                | "plan.cycle_detected"
                | "plan.unknown_tool"
                | "plan.unknown_app"
                | "plan.args_invalid" => {
                    assert_eq!(class, FailureClass::ModelOutput);
                }
                "safety.approval_rejected"
                | "safety.invalid_signature_token"
                | "mcp.so_peercred_failed" => {
                    assert_eq!(class, FailureClass::Safety);
                }
                "db.write_failed" | "db.read_failed" | "ipc.window_not_found" => {
                    assert_eq!(class, FailureClass::Environment);
                }
                "ipc.bad_state" => {
                    assert_eq!(class, FailureClass::User);
                }
                _ => {}
            }
        }
    }
}
