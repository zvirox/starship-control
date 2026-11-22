use crate::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use starship_core::CoreError;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Computes a deterministic SHA-256 hash for an audit log entry chaining from `prev_hash`.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn compute_audit_hash(
    prev_hash: &str,
    mission_id: Option<i64>,
    app_id: &str,
    action_type: &str,
    risk_tier: i64,
    payload_json: &str,
    status: &str,
    signature_token: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(mission_id.unwrap_or(-1).to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(app_id.as_bytes());
    hasher.update(b"|");
    hasher.update(action_type.as_bytes());
    hasher.update(b"|");
    hasher.update(risk_tier.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(payload_json.as_bytes());
    hasher.update(b"|");
    hasher.update(status.as_bytes());
    hasher.update(b"|");
    hasher.update(signature_token.unwrap_or("").as_bytes());
    hex::encode(hasher.finalize())
}

/// Immutable audit record stored in SQLite (DATABASE.md §3.8 & GAP-CLOSURE-PLAN.md §10.8).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuditRecord {
    pub id: i64,
    pub mission_id: Option<i64>,
    pub app_id: String,
    pub action_type: String,
    pub risk_tier: i64,
    pub payload_json: String,
    pub status: String,
    pub signature_token: Option<String>,
    pub prev_hash: String,
    pub entry_hash: String,
    pub created_at: String,
}

/// New audit entry to be recorded in the append-only `audit_log` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAuditEntry {
    pub mission_id: Option<i64>,
    pub app_id: String,
    pub action_type: String,
    pub risk_tier: i64,
    pub payload_json: String,
    pub status: String,
    pub signature_token: Option<String>,
}

impl Database {
    /// Appends an immutable, hash-chained audit record to the `audit_log` table.
    pub fn record_audit_log(&self, entry: &NewAuditEntry) -> Result<i64, CoreError> {
        self.with_conn(|conn| {
            // Find the entry_hash of the most recent audit row, or default to GENESIS_HASH.
            let mut stmt = conn.prepare("SELECT entry_hash FROM audit_log ORDER BY id DESC LIMIT 1")?;
            let prev_hash: String = match stmt.query_row([], |row| row.get(0)) {
                Ok(h) => h,
                Err(rusqlite::Error::QueryReturnedNoRows) => GENESIS_HASH.to_string(),
                Err(e) => return Err(e),
            };

            let entry_hash = compute_audit_hash(
                &prev_hash,
                entry.mission_id,
                &entry.app_id,
                &entry.action_type,
                entry.risk_tier,
                &entry.payload_json,
                &entry.status,
                entry.signature_token.as_deref(),
            );

            conn.execute(
                "INSERT INTO audit_log (
                    mission_id, app_id, action_type, risk_tier, payload_json, status, signature_token, prev_hash, entry_hash
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    entry.mission_id,
                    entry.app_id,
                    entry.action_type,
                    entry.risk_tier,
                    entry.payload_json,
                    entry.status,
                    entry.signature_token,
                    prev_hash,
                    entry_hash,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    /// Fetches audit logs ordered by sequence number descending.
    pub fn list_audit_logs(&self, limit: u32, offset: u32) -> Result<Vec<AuditRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, mission_id, app_id, action_type, risk_tier, payload_json, status, signature_token, prev_hash, entry_hash, created_at
                 FROM audit_log
                 ORDER BY id DESC
                 LIMIT ?1 OFFSET ?2",
            )?;

            let rows = stmt.query_map(params![limit, offset], |row| {
                Ok(AuditRecord {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    app_id: row.get(2)?,
                    action_type: row.get(3)?,
                    risk_tier: row.get(4)?,
                    payload_json: row.get(5)?,
                    status: row.get(6)?,
                    signature_token: row.get(7)?,
                    prev_hash: row.get(8)?,
                    entry_hash: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })?;

            let mut records = Vec::new();
            for r in rows {
                records.push(r?);
            }
            Ok(records)
        })
    }

    /// Walks the audit log chain from id ASC, verifying cryptographic hash continuity (GAP-CLOSURE-PLAN.md §10.8).
    /// Returns the number of validated records on success, or a `db.audit_tampered` `CoreError` identifying the tampered row ID.
    pub fn verify_audit_chain(&self) -> Result<usize, CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT id, mission_id, app_id, action_type, risk_tier, payload_json, status, signature_token, prev_hash, entry_hash
             FROM audit_log
             ORDER BY id ASC",
            )
            .map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Prepare failed: {e}"),
                    None,
                )
            })?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .map_err(|e| {
                CoreError::db("db.query_failed", false, format!("Query failed: {e}"), None)
            })?;

        let mut expected_prev_hash = GENESIS_HASH.to_string();
        let mut count = 0;

        for r in rows {
            let (
                id,
                mission_id,
                app_id,
                action_type,
                risk_tier,
                payload_json,
                status,
                signature_token,
                prev_hash,
                entry_hash,
            ) = r.map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Row fetch failed: {e}"),
                    None,
                )
            })?;

            if prev_hash != expected_prev_hash {
                return Err(CoreError::db(
                    "db.audit_tampered",
                    false,
                    format!(
                        "Tamper detected in audit log at record ID {id}: previous link mismatch."
                    ),
                    Some(serde_json::json!({
                        "id": id,
                        "prev_hash": prev_hash,
                        "expected_prev_hash": expected_prev_hash,
                    })),
                ));
            }

            let computed_hash = compute_audit_hash(
                &prev_hash,
                mission_id,
                &app_id,
                &action_type,
                risk_tier,
                &payload_json,
                &status,
                signature_token.as_deref(),
            );

            if entry_hash != computed_hash {
                return Err(CoreError::db(
                    "db.audit_tampered",
                    false,
                    format!(
                        "Tamper detected in audit log at record ID {id}: cryptographic hash mismatch."
                    ),
                    Some(serde_json::json!({
                        "id": id,
                        "entry_hash": entry_hash,
                        "computed_hash": computed_hash,
                    })),
                ));
            }

            expected_prev_hash = entry_hash;
            count += 1;
        }

        Ok(count)
    }
}
