use crate::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use starship_core::CoreError;

/// Immutable audit record stored in SQLite (DATABASE.md §3.8).
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
    /// Appends an immutable audit record to the `audit_log` table.
    pub fn record_audit_log(&self, entry: &NewAuditEntry) -> Result<i64, CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO audit_log (
                    mission_id, app_id, action_type, risk_tier, payload_json, status, signature_token
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    entry.mission_id,
                    entry.app_id,
                    entry.action_type,
                    entry.risk_tier,
                    entry.payload_json,
                    entry.status,
                    entry.signature_token,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    /// Fetches audit logs ordered by sequence number descending.
    pub fn list_audit_logs(&self, limit: u32, offset: u32) -> Result<Vec<AuditRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, mission_id, app_id, action_type, risk_tier, payload_json, status, signature_token, created_at
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
                    created_at: row.get(8)?,
                })
            })?;

            let mut records = Vec::new();
            for r in rows {
                records.push(r?);
            }
            Ok(records)
        })
    }
}
