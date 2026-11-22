use crate::{CoreError, ErrorKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Active resource lease granted to a mission step (GAP-CLOSURE-PLAN.md §11.4).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceLease {
    pub lease_id: String,
    pub resource_key: String,
    pub mission_id: i64,
    pub step_id: i64,
    pub acquired_at_ms: u64,
    pub expires_at_ms: u64,
    pub lease_timeout_ms: u64,
}

/// Lease and lock supervisor providing deadlock-free acquisition and heartbeat renewals.
#[derive(Debug, Default)]
pub struct LeaseSupervisor {
    leases: Mutex<HashMap<String, ResourceLease>>,
}

impl LeaseSupervisor {
    pub const DEFAULT_LEASE_TIMEOUT_MS: u64 = 10_000;

    #[must_use]
    pub fn new() -> Self {
        Self {
            leases: Mutex::new(HashMap::new()),
        }
    }

    /// Acquires locks for multiple resources in a deadlock-free sorted sequence.
    ///
    /// # Errors
    /// Returns `mcp.resource_locked` if any requested resource is actively locked by another holder.
    pub fn acquire_leases(
        &self,
        mission_id: i64,
        step_id: i64,
        mut resource_keys: Vec<String>,
        lease_timeout_ms: u64,
        now_ms: u64,
    ) -> Result<Vec<ResourceLease>, CoreError> {
        let timeout = if lease_timeout_ms == 0 {
            Self::DEFAULT_LEASE_TIMEOUT_MS
        } else {
            lease_timeout_ms
        };

        // Deadlock-free ordering: canonical sorted acquisition
        resource_keys.sort();
        resource_keys.dedup();

        let mut lock = self.leases.lock().map_err(|_| {
            CoreError::new(
                ErrorKind::Mcp,
                "mcp.lock_poisoned",
                false,
                "Lease supervisor mutex poisoned",
                None,
            )
        })?;

        // 1. Verify availability of all resources first (all-or-nothing check)
        for key in &resource_keys {
            if let Some(existing) = lock.get(key) {
                // If lease has expired, it can be reclaimed
                if now_ms <= existing.expires_at_ms {
                    // If locked by a different mission or step, reject
                    if existing.mission_id != mission_id || existing.step_id != step_id {
                        return Err(CoreError::new(
                            ErrorKind::Mcp,
                            "mcp.resource_locked",
                            true,
                            format!(
                                "Resource '{key}' is currently locked by mission {} step {}",
                                existing.mission_id, existing.step_id
                            ),
                            Some(serde_json::json!({
                                "resourceKey": key,
                                "holderMissionId": existing.mission_id,
                                "holderStepId": existing.step_id,
                                "expiresAtMs": existing.expires_at_ms,
                            })),
                        ));
                    }
                }
            }
        }

        // 2. Grant leases
        let mut granted = Vec::new();
        for (idx, key) in resource_keys.into_iter().enumerate() {
            let lease_id = format!("lse_{mission_id}_{step_id}_{idx}_{now_ms}");
            let lease = ResourceLease {
                lease_id: lease_id.clone(),
                resource_key: key.clone(),
                mission_id,
                step_id,
                acquired_at_ms: now_ms,
                expires_at_ms: now_ms + timeout,
                lease_timeout_ms: timeout,
            };
            lock.insert(key, lease.clone());
            granted.push(lease);
        }

        Ok(granted)
    }

    /// Renews an active lease during heartbeat progress reporting.
    ///
    /// # Errors
    /// Returns `mcp.lease_expired` if the lease has already lapsed past its expiration timestamp.
    /// Returns `mcp.lease_not_found` if no matching lease exists.
    pub fn renew_lease(&self, lease_id: &str, now_ms: u64) -> Result<(), CoreError> {
        let mut lock = self.leases.lock().map_err(|_| {
            CoreError::new(
                ErrorKind::Mcp,
                "mcp.lock_poisoned",
                false,
                "Lease supervisor mutex poisoned",
                None,
            )
        })?;

        let mut found_key = None;
        for (key, lease) in lock.iter() {
            if lease.lease_id == lease_id {
                found_key = Some(key.clone());
                break;
            }
        }

        let Some(key) = found_key else {
            return Err(CoreError::new(
                ErrorKind::Mcp,
                "mcp.lease_not_found",
                false,
                format!("Lease '{lease_id}' not found"),
                None,
            ));
        };

        let Some(lease) = lock.get_mut(&key) else {
            return Err(CoreError::new(
                ErrorKind::Mcp,
                "mcp.lease_not_found",
                false,
                format!("Lease '{lease_id}' not found"),
                None,
            ));
        };
        if now_ms > lease.expires_at_ms {
            return Err(CoreError::new(
                ErrorKind::Mcp,
                "mcp.lease_expired",
                false,
                format!(
                    "Lease '{lease_id}' for resource '{}' expired at {} ms (now: {} ms)",
                    lease.resource_key, lease.expires_at_ms, now_ms
                ),
                Some(serde_json::json!({
                    "leaseId": lease_id,
                    "resourceKey": lease.resource_key,
                    "expiredAt": lease.expires_at_ms,
                    "nowMs": now_ms,
                })),
            ));
        }

        // Renew expiration window
        lease.expires_at_ms = now_ms + lease.lease_timeout_ms;
        Ok(())
    }

    /// Releases all leases held by a mission (or specific step within the mission).
    pub fn release_leases(&self, mission_id: i64, step_id: Option<i64>) -> usize {
        if let Ok(mut lock) = self.leases.lock() {
            let initial_len = lock.len();
            lock.retain(|_, lease| {
                if lease.mission_id != mission_id {
                    return true;
                }
                if let Some(sid) = step_id {
                    lease.step_id != sid
                } else {
                    false
                }
            });
            initial_len - lock.len()
        } else {
            0
        }
    }

    /// Scans for and evicts expired leases, returning them for cancellation cleanup.
    pub fn clean_expired_leases(&self, now_ms: u64) -> Vec<ResourceLease> {
        if let Ok(mut lock) = self.leases.lock() {
            let mut expired = Vec::new();
            lock.retain(|_, lease| {
                if now_ms > lease.expires_at_ms {
                    expired.push(lease.clone());
                    false
                } else {
                    true
                }
            });
            expired
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialization_of_competing_missions_on_same_resource() {
        let supervisor = LeaseSupervisor::new();
        let resource = "eep:screencast_project:track_1".to_string();

        // Mission 1 acquires resource for 10s at t=1000
        let leases_m1 = supervisor
            .acquire_leases(1, 10, vec![resource.clone()], 10_000, 1_000)
            .expect("m1 acquire");
        assert_eq!(leases_m1.len(), 1);

        // Mission 2 attempts to acquire same resource at t=2000 -> Rejected with mcp.resource_locked
        let m2_res = supervisor.acquire_leases(2, 20, vec![resource.clone()], 10_000, 2_000);
        assert!(m2_res.is_err());
        let err = m2_res.unwrap_err();
        assert_eq!(err.code, "mcp.resource_locked");

        // Mission 1 finishes and releases lease
        let released = supervisor.release_leases(1, Some(10));
        assert_eq!(released, 1);

        // Mission 2 retries at t=3000 -> Succeeded
        let leases_m2 = supervisor
            .acquire_leases(2, 20, vec![resource], 10_000, 3_000)
            .expect("m2 acquire");
        assert_eq!(leases_m2.len(), 1);
        assert_eq!(leases_m2[0].mission_id, 2);
    }

    #[test]
    fn test_crashed_holder_lease_expiration_and_reclamation() {
        let supervisor = LeaseSupervisor::new();
        let resource = "alp:voiceover:take_01".to_string();

        // Mission 1 acquires lease for 5s at t=1000 (expires at 6000)
        let leases_m1 = supervisor
            .acquire_leases(1, 10, vec![resource.clone()], 5_000, 1_000)
            .expect("m1 acquire");
        let lease_id = &leases_m1[0].lease_id;

        // Heartbeat renewal at t=3000 extends expiration to 3000 + 5000 = 8000
        supervisor.renew_lease(lease_id, 3_000).expect("renew");

        // Mission 1 crashes and stops sending heartbeats.
        // At t=7000, lease is still active (expires at 8000). Mission 2 rejected.
        assert!(
            supervisor
                .acquire_leases(2, 20, vec![resource.clone()], 5_000, 7_000)
                .is_err()
        );

        // At t=8001, lease has expired!
        // Renewal attempt by zombie process fails with mcp.lease_expired
        let renew_err = supervisor.renew_lease(lease_id, 8_001).unwrap_err();
        assert_eq!(renew_err.code, "mcp.lease_expired");

        // Mission 2 can now acquire the expired resource smoothly
        let m2_res = supervisor
            .acquire_leases(2, 20, vec![resource], 5_000, 8_001)
            .expect("m2 reclaims expired resource");
        assert_eq!(m2_res[0].mission_id, 2);
    }
}
