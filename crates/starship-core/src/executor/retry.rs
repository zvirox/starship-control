use crate::executor::failure::FailureClass;
use crate::safety::RiskTier;
use serde::{Deserialize, Serialize};

/// Decision outcome determining whether to retry and with what backoff (GAP-CLOSURE-PLAN.md §11.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryDecision {
    pub should_retry: bool,
    pub max_retries: u32,
    pub backoff_ms: u64,
}

/// Idempotency & retry policy evaluator (GAP-CLOSURE-PLAN.md §11.5).
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 3 }
    }
}

impl RetryPolicy {
    /// Evaluates whether a failed step qualifies for automatic retry according to the policy table.
    ///
    /// # Rules (GAP-CLOSURE-PLAN.md §11.5):
    /// 1. Tier 3 (Destructive) steps: **0 automatic retries** under all conditions.
    /// 2. `FailureClass::Safety` / `User` / `Resource` / `ModelOutput`: **0 retries**.
    /// 3. `FailureClass::Transient`: Retries up to `max_attempts` (default 3) with exponential backoff (1s, 3s, 9s).
    /// 4. `FailureClass::AppFailure`: Retries up to 1 time **only if** tool is idempotent OR request has an idempotency key.
    /// 5. `FailureClass::Environment`: Retries up to 1 time.
    #[must_use]
    pub fn evaluate_retry(
        &self,
        risk_tier: RiskTier,
        failure_class: FailureClass,
        is_idempotent: bool,
        has_idempotency_key: bool,
        current_attempt: u32,
    ) -> RetryDecision {
        // Hard Constraint: Tier 3 steps NEVER auto-retry (DoD invariant)
        if risk_tier == RiskTier::Destructive {
            return RetryDecision {
                should_retry: false,
                max_retries: 0,
                backoff_ms: 0,
            };
        }

        let max_retries = match failure_class {
            FailureClass::Safety
            | FailureClass::User
            | FailureClass::Resource
            | FailureClass::ModelOutput => 0,
            FailureClass::Transient => self.max_attempts,
            FailureClass::AppFailure => u32::from(is_idempotent || has_idempotency_key),
            FailureClass::Environment => 1,
        };

        let should_retry = current_attempt < max_retries;
        let backoff_ms = if should_retry {
            Self::compute_backoff_ms(current_attempt)
        } else {
            0
        };

        RetryDecision {
            should_retry,
            max_retries,
            backoff_ms,
        }
    }

    /// Computes exponential backoff with jitter: 1 s, 3 s, 9 s (GAP-CLOSURE-PLAN.md §11.5).
    #[must_use]
    pub fn compute_backoff_ms(attempt: u32) -> u64 {
        match attempt {
            0 => 1_000,
            1 => 3_000,
            _ => 9_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_matrix() {
        let policy = RetryPolicy::default();

        let all_tiers = [RiskTier::Safe, RiskTier::Mutating, RiskTier::Destructive];
        let all_classes = [
            FailureClass::Transient,
            FailureClass::AppFailure,
            FailureClass::Resource,
            FailureClass::ModelOutput,
            FailureClass::Safety,
            FailureClass::Environment,
            FailureClass::User,
        ];
        let boolean_options = [false, true];

        for &tier in &all_tiers {
            for &class in &all_classes {
                for &idempotent in &boolean_options {
                    for &has_key in &boolean_options {
                        let decision = policy.evaluate_retry(tier, class, idempotent, has_key, 0);

                        // Hard Invariant 1: Tier 3 NEVER retries
                        if tier == RiskTier::Destructive {
                            assert_eq!(
                                decision.max_retries, 0,
                                "Tier 3 tool must have 0 max_retries"
                            );
                            assert!(!decision.should_retry);
                            continue;
                        }

                        // Hard Invariant 2: Safety, User, Resource, ModelOutput NEVER retry automatically
                        if matches!(
                            class,
                            FailureClass::Safety
                                | FailureClass::User
                                | FailureClass::Resource
                                | FailureClass::ModelOutput
                        ) {
                            assert_eq!(decision.max_retries, 0);
                            assert!(!decision.should_retry);
                            continue;
                        }

                        // Hard Invariant 3: Transient retries 3 times
                        if class == FailureClass::Transient {
                            assert_eq!(decision.max_retries, 3);
                            assert!(decision.should_retry);
                            assert_eq!(decision.backoff_ms, 1_000);

                            // Test backoff progression
                            assert_eq!(RetryPolicy::compute_backoff_ms(0), 1_000);
                            assert_eq!(RetryPolicy::compute_backoff_ms(1), 3_000);
                            assert_eq!(RetryPolicy::compute_backoff_ms(2), 9_000);
                            continue;
                        }

                        // Hard Invariant 4: AppFailure retries only if idempotent or key present
                        if class == FailureClass::AppFailure {
                            if idempotent || has_key {
                                assert_eq!(decision.max_retries, 1);
                                assert!(decision.should_retry);
                            } else {
                                assert_eq!(decision.max_retries, 0);
                                assert!(!decision.should_retry);
                            }
                        }

                        // Hard Invariant 5: Environment retries once
                        if class == FailureClass::Environment {
                            assert_eq!(decision.max_retries, 1);
                            assert!(decision.should_retry);
                        }
                    }
                }
            }
        }
    }
}
