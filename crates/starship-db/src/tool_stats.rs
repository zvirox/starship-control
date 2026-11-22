use crate::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use starship_core::CoreError;

/// Maximum window of historical samples kept for exact percentile estimation.
pub const MAX_SAMPLES_WINDOW: usize = 100;

/// Standard exponential moving average weight (alpha = 0.2).
pub const EWMA_ALPHA: f64 = 0.2;

/// Persisted execution metrics for a tool (GAP-CLOSURE-PLAN.md §13.3 & Appendix B).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolStatsRecord {
    pub app_id: String,
    pub tool: String,
    pub n: usize,
    pub ewma_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
}

impl Database {
    /// Retrieves historical execution stats for a specific tool.
    pub fn get_tool_stats(
        &self,
        app_id: &str,
        tool: &str,
    ) -> Result<Option<ToolStatsRecord>, CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT app_id, tool, n, ewma_ms, p50_ms, p90_ms
                 FROM tool_stats WHERE app_id = ?1 AND tool = ?2",
            )
            .map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Failed to prepare get_tool_stats: {e}"),
                    None,
                )
            })?;

        let mut rows = stmt.query(params![app_id, tool]).map_err(|e| {
            CoreError::db(
                "db.query_failed",
                false,
                format!("Failed to query tool_stats: {e}"),
                None,
            )
        })?;

        if let Some(row) = rows.next().map_err(|e| {
            CoreError::db(
                "db.query_failed",
                false,
                format!("Failed to fetch tool_stats row: {e}"),
                None,
            )
        })? {
            let n_i64: i64 = row.get(2).unwrap_or(0);
            let n = usize::try_from(n_i64.max(0)).unwrap_or(0);
            Ok(Some(ToolStatsRecord {
                app_id: row.get(0).unwrap_or_default(),
                tool: row.get(1).unwrap_or_default(),
                n,
                ewma_ms: row.get(3).unwrap_or(0.0),
                p50_ms: row.get(4).unwrap_or(0.0),
                p90_ms: row.get(5).unwrap_or(0.0),
            }))
        } else {
            Ok(None)
        }
    }

    /// Lists all recorded tool stats in the catalog.
    pub fn list_all_tool_stats(&self) -> Result<Vec<ToolStatsRecord>, CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT app_id, tool, n, ewma_ms, p50_ms, p90_ms
                 FROM tool_stats ORDER BY tool ASC",
            )
            .map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Failed to prepare list_all_tool_stats: {e}"),
                    None,
                )
            })?;

        let rows = stmt
            .query_map([], |row| {
                let n_i64: i64 = row.get(2)?;
                let n = usize::try_from(n_i64.max(0)).unwrap_or(0);
                Ok(ToolStatsRecord {
                    app_id: row.get(0)?,
                    tool: row.get(1)?,
                    n,
                    ewma_ms: row.get(3)?,
                    p50_ms: row.get(4)?,
                    p90_ms: row.get(5)?,
                })
            })
            .map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Failed to query list_all_tool_stats: {e}"),
                    None,
                )
            })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Failed to decode tool_stats row: {e}"),
                    None,
                )
            })?);
        }

        Ok(list)
    }

    /// Records a step execution duration. If `is_success` is false, update is ignored (GAP-CLOSURE-PLAN.md §13.3).
    pub fn record_tool_duration(
        &self,
        app_id: &str,
        tool: &str,
        duration_ms: f64,
        is_success: bool,
    ) -> Result<Option<ToolStatsRecord>, CoreError> {
        // Requirement: "Ignore failed/cancelled runs"
        if !is_success {
            return self.get_tool_stats(app_id, tool);
        }

        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        // 1. Fetch existing samples and stats
        let existing: Option<(i64, f64, String)> = conn
            .query_row(
                "SELECT n, ewma_ms, samples_json FROM tool_stats WHERE app_id = ?1 AND tool = ?2",
                params![app_id, tool],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        let (new_n, new_ewma, samples) = if let Some((prev_n, prev_ewma, samples_raw)) = existing {
            let mut s: Vec<f64> = serde_json::from_str(&samples_raw).unwrap_or_default();
            s.push(duration_ms);
            if s.len() > MAX_SAMPLES_WINDOW {
                s.remove(0);
            }
            let n = usize::try_from((prev_n + 1).max(1)).unwrap_or(1);
            let ewma = (1.0 - EWMA_ALPHA) * prev_ewma + EWMA_ALPHA * duration_ms;
            (n, ewma, s)
        } else {
            (1, duration_ms, vec![duration_ms])
        };

        // 2. Compute exact empirical quantiles from samples window
        let mut sorted_samples = samples.clone();
        sorted_samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p50_idx = ((sorted_samples.len() * 50) / 100).min(sorted_samples.len() - 1);
        let p90_idx = ((sorted_samples.len() * 90) / 100).min(sorted_samples.len() - 1);

        let p50_ms = sorted_samples[p50_idx];
        let p90_ms = sorted_samples[p90_idx];

        let samples_json = serde_json::to_string(&samples).unwrap_or_else(|_| "[]".to_string());

        // 3. Upsert row
        #[allow(clippy::cast_possible_wrap)]
        let new_n_i64 = new_n as i64;

        conn.execute(
            "INSERT INTO tool_stats (app_id, tool, n, ewma_ms, p50_ms, p90_ms, samples_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(app_id, tool) DO UPDATE SET
               n = excluded.n,
               ewma_ms = excluded.ewma_ms,
               p50_ms = excluded.p50_ms,
               p90_ms = excluded.p90_ms,
               samples_json = excluded.samples_json",
            params![
                app_id,
                tool,
                new_n_i64,
                new_ewma,
                p50_ms,
                p90_ms,
                samples_json
            ],
        )
        .map_err(|e| {
            CoreError::db(
                "db.write_failed",
                false,
                format!("Failed to upsert tool_stats: {e}"),
                None,
            )
        })?;

        Ok(Some(ToolStatsRecord {
            app_id: app_id.to_string(),
            tool: tool.to_string(),
            n: new_n,
            ewma_ms: new_ewma,
            p50_ms,
            p90_ms,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_stats_learner_ignores_failures_and_learns_accurate_eta() {
        let db = Database::open_in_memory().expect("open in-memory db");

        // 1. Initial lookup returns None
        assert!(
            db.get_tool_stats("alp", "alp.denoise.spectral")
                .unwrap()
                .is_none()
        );

        // 2. Failed run is strictly ignored
        let ignored = db
            .record_tool_duration("alp", "alp.denoise.spectral", 10000.0, false)
            .unwrap();
        assert!(ignored.is_none());
        assert!(
            db.get_tool_stats("alp", "alp.denoise.spectral")
                .unwrap()
                .is_none()
        );

        // 3. Replay 10 successful runs with true duration ~ 2000ms
        let simulated_durations = [
            2100.0, 1950.0, 2050.0, 1980.0, 2020.0, 1920.0, 2080.0, 2010.0, 1990.0, 2000.0,
        ];

        let mut final_stats = None;
        for duration in simulated_durations {
            final_stats = db
                .record_tool_duration("alp", "alp.denoise.spectral", duration, true)
                .unwrap();
        }

        let stats = final_stats.expect("has stats after 10 runs");
        assert_eq!(stats.n, 10);

        // Verification requirement for 13.3:
        // "ETA error on a replayed history < 25 % after 10 runs (record result)"
        let true_duration = 2000.0;
        let estimated_eta = stats.p50_ms;
        let error_pct = ((estimated_eta - true_duration).abs() / true_duration) * 100.0;

        println!(
            "Tool stats after 10 runs: n={}, ewma={:.1}ms, p50={:.1}ms, p90={:.1}ms, error={:.2}%",
            stats.n, stats.ewma_ms, stats.p50_ms, stats.p90_ms, error_pct
        );

        assert!(
            error_pct < 25.0,
            "Expected ETA error < 25%, actual was {error_pct:.2}%"
        );
        // In fact, with median around 2005ms, error is < 1%!
        assert!(error_pct < 2.0);
    }
}
