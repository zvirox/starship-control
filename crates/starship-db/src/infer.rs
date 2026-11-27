use crate::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use std::time::{SystemTime, UNIX_EPOCH};

/// One measured inference request (Phase 17, task 17.5). `tokensOut` counts
/// non-empty streamed chunks — the providers used here do not return a token
/// count, so no value is invented.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferMetricRecord {
    pub id: i64,
    pub ts_ms: i64,
    pub mission_id: Option<i64>,
    pub provider: String,
    pub model_id: String,
    pub first_token_ms: i64,
    pub total_latency_ms: i64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub tokens_per_sec: f64,
    pub error_code: Option<String>,
}

/// Current UTC epoch milliseconds (used by telemetry records).
#[must_use]
pub fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(i64::MAX)
}

impl Database {
    /// Stores a measured inference request.
    pub fn record_infer_metric(&self, metric: &InferMetricRecord) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO infer_metrics
                 (ts_ms, mission_id, provider, model_id, first_token_ms,
                  total_latency_ms, tokens_in, tokens_out, tokens_per_sec, error_code)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    now_ms(),
                    metric.mission_id,
                    metric.provider,
                    metric.model_id,
                    metric.first_token_ms,
                    metric.total_latency_ms,
                    metric.tokens_in,
                    metric.tokens_out,
                    metric.tokens_per_sec,
                    metric.error_code,
                ],
            )?;
            Ok(())
        })
    }

    /// Returns recent measured requests for `range`: `15m` (default), `1h`, `24h`.
    pub fn list_infer_metrics(&self, range: &str) -> Result<Vec<InferMetricRecord>, CoreError> {
        let window_ms = match range {
            "1h" => 3_600_000_i64,
            "24h" => 86_400_000_i64,
            _ => 900_000_i64,
        };
        let cutoff = now_ms().saturating_sub(window_ms);
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, ts_ms, mission_id, provider, model_id, first_token_ms,
                        total_latency_ms, tokens_in, tokens_out, tokens_per_sec, error_code
                 FROM infer_metrics WHERE ts_ms >= ?1 ORDER BY ts_ms DESC LIMIT 500",
            )?;
            let rows = stmt.query_map([cutoff], |row| {
                Ok(InferMetricRecord {
                    id: row.get(0)?,
                    ts_ms: row.get(1)?,
                    mission_id: row.get(2)?,
                    provider: row.get(3)?,
                    model_id: row.get(4)?,
                    first_token_ms: row.get(5)?,
                    total_latency_ms: row.get(6)?,
                    tokens_in: row.get(7)?,
                    tokens_out: row.get(8)?,
                    tokens_per_sec: row.get(9)?,
                    error_code: row.get(10)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()
        })
    }
}
