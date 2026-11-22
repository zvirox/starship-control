use crate::CoreError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

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

/// Cryptographically bound approval request (GAP-CLOSURE-PLAN.md §10.3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApprovalRequest {
    pub id: String,
    pub plan_id: i64,
    pub step_id: i64,
    pub nonce: String,
    pub args_hash: String,
    pub risk_tier: RiskTier,
    pub expires_at: u64,
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
    /// Host-enforced tier policy (GAP-CLOSURE-PLAN.md §10.1).
    tier_policy: crate::policy::TierPolicy,
    /// Active pending approval requests keyed by single-use nonce.
    active_requests: Arc<Mutex<HashMap<String, ApprovalRequest>>>,
    /// Set of consumed nonces to strictly prevent replay attacks.
    consumed_nonces: Arc<Mutex<HashSet<String>>>,
}

impl Default for SafetyArbiter {
    fn default() -> Self {
        Self::new("starship-control-default-safety-salt-2026")
    }
}

impl SafetyArbiter {
    /// Creates a new `SafetyArbiter` with a given secret salt and default tier policy.
    #[must_use]
    pub fn new(secret_salt: impl Into<String>) -> Self {
        Self {
            secret_salt: secret_salt.into(),
            tier_policy: crate::policy::TierPolicy::default(),
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            consumed_nonces: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Creates a new `SafetyArbiter` with a custom tier policy.
    #[must_use]
    pub fn with_policy(
        secret_salt: impl Into<String>,
        tier_policy: crate::policy::TierPolicy,
    ) -> Self {
        Self {
            secret_salt: secret_salt.into(),
            tier_policy,
            active_requests: Arc::new(Mutex::new(HashMap::new())),
            consumed_nonces: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Computes canonical SHA-256 hash of step arguments.
    #[must_use]
    pub fn compute_args_hash(arguments: &Value) -> String {
        let canonical = serde_json::to_string(arguments).unwrap_or_else(|_| "{}".to_string());
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Creates an argument-bound approval request with a 5-minute single-use nonce (GAP-CLOSURE-PLAN.md §10.3).
    #[must_use]
    pub fn create_approval_request(
        &self,
        plan_id: i64,
        step_id: i64,
        arguments: &Value,
        risk_tier: RiskTier,
        now_secs: u64,
    ) -> ApprovalRequest {
        let args_hash = Self::compute_args_hash(arguments);

        let mut hasher = Sha256::new();
        hasher.update(b"starship-approval-nonce-v2:");
        hasher.update(plan_id.to_le_bytes());
        hasher.update(step_id.to_le_bytes());
        hasher.update(args_hash.as_bytes());
        hasher.update(now_secs.to_le_bytes());
        hasher.update(self.secret_salt.as_bytes());
        let nonce_hash = hex::encode(&hasher.finalize()[..16]);
        let nonce = format!("usr_nonce_{nonce_hash}");

        let request = ApprovalRequest {
            id: format!("apr_{nonce_hash}"),
            plan_id,
            step_id,
            nonce: nonce.clone(),
            args_hash,
            risk_tier,
            expires_at: now_secs + 300, // 5 minute TTL
        };

        if let Ok(mut lock) = self.active_requests.lock() {
            lock.insert(nonce, request.clone());
        }

        request
    }

    /// Verifies and consumes a single-use approval token against current arguments.
    ///
    /// # Errors
    /// Returns `safety.invalid_signature_token` on replay, expiration, argument mutation, or wrong step.
    pub fn verify_and_consume_approval(
        &self,
        plan_id: i64,
        step_id: i64,
        current_arguments: &Value,
        token_or_nonce: &str,
        now_secs: u64,
    ) -> Result<(), CoreError> {
        // 1. Replay check: has this token/nonce already been consumed?
        if let Ok(consumed) = self.consumed_nonces.lock()
            && consumed.contains(token_or_nonce)
        {
            return Err(CoreError::safety(
                "safety.invalid_signature_token",
                false,
                format!(
                    "Approval token '{token_or_nonce}' has already been consumed (replay prevented)"
                ),
                Some(json!({ "planId": plan_id, "stepId": step_id, "nonce": token_or_nonce })),
            ));
        }

        // 2. Active request lookup
        let req = {
            let mut active = self.active_requests.lock().map_err(|_| {
                CoreError::safety(
                    "safety.lock_error",
                    false,
                    "Failed to acquire safety lock",
                    None,
                )
            })?;
            active.remove(token_or_nonce)
        };

        let Some(request) = req else {
            // Check legacy token format fallback if applicable
            return self.verify_legacy_signature_token(plan_id, step_id, token_or_nonce);
        };

        // 3. Expiration check (5 minute TTL)
        if now_secs > request.expires_at {
            return Err(CoreError::safety(
                "safety.invalid_signature_token",
                false,
                format!(
                    "Approval token expired at {} (current time: {})",
                    request.expires_at, now_secs
                ),
                Some(
                    json!({ "planId": plan_id, "stepId": step_id, "expiresAt": request.expires_at }),
                ),
            ));
        }

        // 4. Step binding check
        if request.plan_id != plan_id || request.step_id != step_id {
            return Err(CoreError::safety(
                "safety.invalid_signature_token",
                false,
                format!(
                    "Approval token issued for (plan {}, step {}) does not match requested (plan {}, step {})",
                    request.plan_id, request.step_id, plan_id, step_id
                ),
                Some(json!({ "expectedStep": request.step_id, "actualStep": step_id })),
            ));
        }

        // 5. Argument mutation check (re-hash current arguments against approved args_hash)
        let current_args_hash = Self::compute_args_hash(current_arguments);
        if current_args_hash != request.args_hash {
            return Err(CoreError::safety(
                "safety.invalid_signature_token",
                false,
                format!(
                    "Step arguments were mutated after approval was granted for step {step_id}"
                ),
                Some(json!({
                    "approvedHash": request.args_hash,
                    "currentHash": current_args_hash,
                })),
            ));
        }

        // 6. Record nonce as consumed
        if let Ok(mut consumed) = self.consumed_nonces.lock() {
            consumed.insert(token_or_nonce.to_string());
        }

        Ok(())
    }

    /// Verifies legacy non-bound signature token.
    fn verify_legacy_signature_token(
        &self,
        plan_id: i64,
        step_id: i64,
        token: &str,
    ) -> Result<(), CoreError> {
        let expected = self.generate_signature_token(plan_id, step_id, "");
        if token == expected {
            Ok(())
        } else {
            Err(CoreError::safety(
                "safety.invalid_signature_token",
                false,
                format!("Invalid or unknown approval token for step {step_id}"),
                Some(json!({ "stepId": step_id, "token": token })),
            ))
        }
    }

    /// Automatically classifies a tool identifier into its host floor `RiskTier`.
    #[must_use]
    pub fn classify_tool(&self, tool_name: &str) -> RiskTier {
        self.tier_policy.host_floor(tool_name)
    }

    /// Calculates the effective risk tier combining an application-declared tier with host floor.
    /// Invariant: Apps can raise risk tiers, never lower them.
    #[must_use]
    pub fn effective_tier(&self, app_declared: Option<RiskTier>, tool_name: &str) -> RiskTier {
        self.tier_policy
            .calculate_effective_tier(app_declared, tool_name)
    }

    /// Generates a legacy cryptographic signature token proving human consent for a specific plan step.
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

    /// Evaluates execution permission for a step with argument binding support.
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_step_bound(
        &self,
        plan_id: i64,
        step_id: i64,
        tool_name: &str,
        risk_tier: RiskTier,
        arguments: &Value,
        token_or_nonce: Option<&str>,
        now_secs: u64,
    ) -> Result<ExecutionPermission, CoreError> {
        match risk_tier {
            RiskTier::Safe => Ok(ExecutionPermission::Autonomous),
            RiskTier::Mutating | RiskTier::Destructive => {
                let token = token_or_nonce.ok_or_else(|| {
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

                self.verify_and_consume_approval(plan_id, step_id, arguments, token, now_secs)?;
                Ok(ExecutionPermission::Approved {
                    signature_token: token.to_string(),
                })
            }
        }
    }

    /// Evaluates execution permission for a step (legacy API).
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
