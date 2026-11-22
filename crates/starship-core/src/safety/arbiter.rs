use crate::CoreError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

/// Three-tiered safety classification defined in PRD.md §6.4 and AGENTS.md §3.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RiskTier {
    /// Tier 1: Read-only status checks, telemetry queries, and non-destructive inspections.
    Safe = 1,
    /// Tier 2: Mutating operations that modify project state, apply edits, or spend API quota.
    Mutating = 2,
    /// Tier 3: Destructive operations that delete files, purge caches, or overwrite media.
    Destructive = 3,
}

impl RiskTier {
    #[must_use]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Safe),
            2 => Some(Self::Mutating),
            3 => Some(Self::Destructive),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub fn requires_approval(self) -> bool {
        self >= Self::Mutating
    }
}

/// Result of safety evaluation on a proposed tool execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionPermission {
    /// Safe to execute autonomously without human intervention.
    Autonomous,
    /// Mutating or destructive step authorized by verified human signature token.
    Approved { signature_token: String },
}

/// The Safety Arbiter enforces Tiered Safety policy in native Rust code (AGENTS.md §3.2).
#[derive(Debug, Clone)]
pub struct SafetyArbiter {
    /// Cryptographic salt used for signature token verification.
    secret_salt: String,
}

impl Default for SafetyArbiter {
    fn default() -> Self {
        Self::new("starship-control-default-safety-salt-2026")
    }
}

impl SafetyArbiter {
    /// Creates a new `SafetyArbiter` with a given secret salt.
    #[must_use]
    pub fn new(secret_salt: impl Into<String>) -> Self {
        Self {
            secret_salt: secret_salt.into(),
        }
    }

    /// Automatically classifies a tool identifier into its default `RiskTier`
    /// based on canonical naming conventions.
    #[must_use]
    pub fn classify_tool(&self, tool_name: &str) -> RiskTier {
        let lower = tool_name.to_ascii_lowercase();

        // Destructive keywords
        if lower.contains("delete")
            || lower.contains("purge")
            || lower.contains("remove")
            || lower.contains("wipe")
            || lower.contains("destroy")
            || lower.contains("drop")
        {
            return RiskTier::Destructive;
        }

        // Read-only keywords
        if lower.contains("get")
            || lower.contains("list")
            || lower.contains("read")
            || lower.contains("inspect")
            || lower.contains("scan")
            || lower.contains("status")
            || lower.contains("meter")
            || lower.contains("query")
            || lower.contains("find")
            || lower.contains("ping")
        {
            return RiskTier::Safe;
        }

        // Default all other operations to Mutating (Tier 2) for safety
        RiskTier::Mutating
    }

    /// Generates a cryptographic signature token proving human consent for a specific plan step.
    /// Format: `usr_sig_<16-byte-hex>`.
    #[must_use]
    pub fn generate_signature_token(&self, plan_id: i64, step_id: i64, tool_name: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"starship-approval-v1:");
        hasher.update(plan_id.to_le_bytes());
        hasher.update(step_id.to_le_bytes());
        hasher.update(tool_name.as_bytes());
        hasher.update(self.secret_salt.as_bytes());
        let result = hasher.finalize();
        // Use first 16 bytes for a clean 32-character hex token
        let hex_str = hex::encode(&result[..16]);
        format!("usr_sig_{hex_str}")
    }

    /// Verifies that a provided signature token matches the expected cryptographic signature
    /// for the specified `plan_id`, `step_id`, and `tool_name`.
    pub fn verify_signature_token(
        &self,
        plan_id: i64,
        step_id: i64,
        tool_name: &str,
        token: &str,
    ) -> Result<(), CoreError> {
        let expected = self.generate_signature_token(plan_id, step_id, tool_name);
        if token == expected {
            Ok(())
        } else {
            Err(CoreError::safety(
                "safety.invalid_signature_token",
                false,
                format!("Invalid or tampered signature token for step {step_id}"),
                Some(json!({
                    "stepId": step_id,
                    "toolName": tool_name,
                })),
            ))
        }
    }

    /// Evaluates execution permission for a step.
    /// - Tier 1: Always autonomous.
    /// - Tier 2/3: Requires a verified signature token; returns `safety.permission_denied`
    ///   if unapproved, or `safety.invalid_signature_token` if tampered.
    pub fn evaluate_step(
        &self,
        plan_id: i64,
        step_id: i64,
        tool_name: &str,
        risk_tier: RiskTier,
        signature_token: Option<&str>,
    ) -> Result<ExecutionPermission, CoreError> {
        match risk_tier {
            RiskTier::Safe => Ok(ExecutionPermission::Autonomous),
            RiskTier::Mutating | RiskTier::Destructive => {
                let token = signature_token.ok_or_else(|| {
                    CoreError::safety(
                        "safety.permission_denied",
                        false,
                        format!(
                            "Execution blocked: Step {step_id} ({tool_name}) requires explicit human approval token"
                        ),
                        Some(json!({
                            "planId": plan_id,
                            "stepId": step_id,
                            "toolName": tool_name,
                            "riskTier": risk_tier.as_u8(),
                        })),
                    )
                })?;

                self.verify_signature_token(plan_id, step_id, tool_name, token)?;
                Ok(ExecutionPermission::Approved {
                    signature_token: token.to_string(),
                })
            }
        }
    }
}
