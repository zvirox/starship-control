use crate::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use starship_core::error::CoreError;
use tracing::debug;

/// 1-minute and 1-hour rollup metrics record (GAP-CLOSURE-PLAN.md §14.5 & Appendix B).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthRollupRecord {
    pub ts: i64,
    pub cpu_avg: f64,
    pub cpu_max: f64,
    pub mem_avail_pct_min: f64,
    pub swap_in_max: f64,
    pub disk_free_pct_min: f64,
    pub temp_max: f64,
    pub gpu_util_max: Option<f64>,
    pub vram_pct_max: Option<f64>,
    pub psi_cpu_max: Option<f64>,
    pub psi_mem_max: Option<f64>,
    pub psi_io_max: Option<f64>,
    pub state: String,
}

/// Recorded system health alert event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthAlertRecord {
    pub id: i64,
    pub ts: i64,
    pub signal: String,
    pub level: String,
    pub value: f64,
    pub action: String,
    pub mission_id: Option<i64>,
}

impl Database {
    /// Inserts a 1-minute rollup row into `health_rollup_1m`.
    pub fn insert_health_rollup_1m(&self, r: &HealthRollupRecord) -> Result<(), CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO health_rollup_1m (
                ts, cpu_avg, cpu_max, mem_avail_pct_min, swap_in_max,
                disk_free_pct_min, temp_max, gpu_util_max, vram_pct_max,
                psi_cpu_max, psi_mem_max, psi_io_max, state
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                r.ts,
                r.cpu_avg,
                r.cpu_max,
                r.mem_avail_pct_min,
                r.swap_in_max,
                r.disk_free_pct_min,
                r.temp_max,
                r.gpu_util_max,
                r.vram_pct_max,
                r.psi_cpu_max,
                r.psi_mem_max,
                r.psi_io_max,
                r.state,
            ],
        )
        .map_err(|e| {
            CoreError::db(
                "db.write_failed",
                false,
                format!("Failed to insert 1m health rollup: {e}"),
                None,
            )
        })?;

        Ok(())
    }

    /// Inserts a 1-hour rollup row into `health_rollup_1h`.
    pub fn insert_health_rollup_1h(&self, r: &HealthRollupRecord) -> Result<(), CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO health_rollup_1h (
                ts, cpu_avg, cpu_max, mem_avail_pct_min, swap_in_max,
                disk_free_pct_min, temp_max, gpu_util_max, vram_pct_max,
                psi_cpu_max, psi_mem_max, psi_io_max, state
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                r.ts,
                r.cpu_avg,
                r.cpu_max,
                r.mem_avail_pct_min,
                r.swap_in_max,
                r.disk_free_pct_min,
                r.temp_max,
                r.gpu_util_max,
                r.vram_pct_max,
                r.psi_cpu_max,
                r.psi_mem_max,
                r.psi_io_max,
                r.state,
            ],
        )
        .map_err(|e| {
            CoreError::db(
                "db.write_failed",
                false,
                format!("Failed to insert 1h health rollup: {e}"),
                None,
            )
        })?;

        Ok(())
    }

    /// Fetches 1-minute health rollups within the given timestamp range.
    pub fn get_health_rollups_1m(
        &self,
        from_ts: i64,
        to_ts: i64,
    ) -> Result<Vec<HealthRollupRecord>, CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT ts, cpu_avg, cpu_max, mem_avail_pct_min, swap_in_max,
                        disk_free_pct_min, temp_max, gpu_util_max, vram_pct_max,
                        psi_cpu_max, psi_mem_max, psi_io_max, state
                 FROM health_rollup_1m
                 WHERE ts >= ?1 AND ts <= ?2
                 ORDER BY ts ASC",
            )
            .map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Failed to prepare 1m rollups query: {e}"),
                    None,
                )
            })?;

        let rows = stmt
            .query_map(params![from_ts, to_ts], |row| {
                Ok(HealthRollupRecord {
                    ts: row.get(0)?,
                    cpu_avg: row.get(1)?,
                    cpu_max: row.get(2)?,
                    mem_avail_pct_min: row.get(3)?,
                    swap_in_max: row.get(4)?,
                    disk_free_pct_min: row.get(5)?,
                    temp_max: row.get(6)?,
                    gpu_util_max: row.get(7)?,
                    vram_pct_max: row.get(8)?,
                    psi_cpu_max: row.get(9)?,
                    psi_mem_max: row.get(10)?,
                    psi_io_max: row.get(11)?,
                    state: row.get(12)?,
                })
            })
            .map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Failed to execute 1m rollups query: {e}"),
                    None,
                )
            })?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| {
                CoreError::db(
                    "db.read_failed",
                    false,
                    format!("Failed to map 1m rollup row: {e}"),
                    None,
                )
            })?);
        }

        Ok(result)
    }

    /// Fetches 1-hour health rollups within the given timestamp range.
    pub fn get_health_rollups_1h(
        &self,
        from_ts: i64,
        to_ts: i64,
    ) -> Result<Vec<HealthRollupRecord>, CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        let mut stmt = conn
            .prepare(
                "SELECT ts, cpu_avg, cpu_max, mem_avail_pct_min, swap_in_max,
                        disk_free_pct_min, temp_max, gpu_util_max, vram_pct_max,
                        psi_cpu_max, psi_mem_max, psi_io_max, state
                 FROM health_rollup_1h
                 WHERE ts >= ?1 AND ts <= ?2
                 ORDER BY ts ASC",
            )
            .map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Failed to prepare 1h rollups query: {e}"),
                    None,
                )
            })?;

        let rows = stmt
            .query_map(params![from_ts, to_ts], |row| {
                Ok(HealthRollupRecord {
                    ts: row.get(0)?,
                    cpu_avg: row.get(1)?,
                    cpu_max: row.get(2)?,
                    mem_avail_pct_min: row.get(3)?,
                    swap_in_max: row.get(4)?,
                    disk_free_pct_min: row.get(5)?,
                    temp_max: row.get(6)?,
                    gpu_util_max: row.get(7)?,
                    vram_pct_max: row.get(8)?,
                    psi_cpu_max: row.get(9)?,
                    psi_mem_max: row.get(10)?,
                    psi_io_max: row.get(11)?,
                    state: row.get(12)?,
                })
            })
            .map_err(|e| {
                CoreError::db(
                    "db.query_failed",
                    false,
                    format!("Failed to execute 1h rollups query: {e}"),
                    None,
                )
            })?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r.map_err(|e| {
                CoreError::db(
                    "db.read_failed",
                    false,
                    format!("Failed to map 1h rollup row: {e}"),
                    None,
                )
            })?);
        }

        Ok(result)
    }

    /// Records an overload alert event.
    pub fn record_health_alert(
        &self,
        ts: i64,
        signal: &str,
        level: &str,
        value: f64,
        action: &str,
        mission_id: Option<i64>,
    ) -> Result<i64, CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        conn.execute(
            "INSERT INTO health_alerts (ts, signal, level, value, action, mission_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![ts, signal, level, value, action, mission_id],
        )
        .map_err(|e| {
            CoreError::db(
                "db.write_failed",
                false,
                format!("Failed to insert health alert: {e}"),
                None,
            )
        })?;

        Ok(conn.last_insert_rowid())
    }

    /// Prune old health rollups:
    /// - `health_rollup_1m`: older than 7 days (7 * 86,400 seconds)
    /// - `health_rollup_1h`: older than 90 days (90 * 86,400 seconds)
    ///
    /// Returns `(pruned_1m_count, pruned_1h_count)`.
    pub fn prune_health_rollups(&self, now_ts: i64) -> Result<(usize, usize), CoreError> {
        let conn = self.conn.lock().map_err(|_| {
            CoreError::db("db.lock_poisoned", false, "Database lock poisoned", None)
        })?;

        let cutoff_1m = now_ts - (7 * 86_400);
        let cutoff_1h = now_ts - (90 * 86_400);

        let pruned_1m = conn
            .execute(
                "DELETE FROM health_rollup_1m WHERE ts < ?1",
                params![cutoff_1m],
            )
            .map_err(|e| {
                CoreError::db(
                    "db.delete_failed",
                    false,
                    format!("Failed to prune 1m rollups: {e}"),
                    None,
                )
            })?;

        let pruned_1h = conn
            .execute(
                "DELETE FROM health_rollup_1h WHERE ts < ?1",
                params![cutoff_1h],
            )
            .map_err(|e| {
                CoreError::db(
                    "db.delete_failed",
                    false,
                    format!("Failed to prune 1h rollups: {e}"),
                    None,
                )
            })?;

        debug!(
            target: "starship_db",
            pruned_1m,
            pruned_1h,
            cutoff_1m,
            cutoff_1h,
            "Pruned health rollups"
        );

        Ok((pruned_1m, pruned_1h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_rollups_storage_and_prune_lifecycle() {
        let db = Database::open_in_memory().expect("in-memory db");
        let now_ts = 1_700_000_000;

        // 1. Insert 1-minute rollups across multiple simulated days
        // Insert a recent row (1 day ago) and an expired row (8 days ago)
        let recent_1m = HealthRollupRecord {
            ts: now_ts - 86_400,
            cpu_avg: 42.5,
            cpu_max: 65.0,
            mem_avail_pct_min: 35.0,
            swap_in_max: 0.0,
            disk_free_pct_min: 60.0,
            temp_max: 52.0,
            gpu_util_max: Some(30.0),
            vram_pct_max: Some(25.0),
            psi_cpu_max: Some(0.1),
            psi_mem_max: Some(0.0),
            psi_io_max: Some(0.05),
            state: "Normal".to_string(),
        };

        let expired_1m = HealthRollupRecord {
            ts: now_ts - (8 * 86_400), // 8 days old (> 7 days)
            cpu_avg: 50.0,
            cpu_max: 80.0,
            mem_avail_pct_min: 20.0,
            swap_in_max: 1024.0,
            disk_free_pct_min: 55.0,
            temp_max: 65.0,
            gpu_util_max: None,
            vram_pct_max: None,
            psi_cpu_max: None,
            psi_mem_max: None,
            psi_io_max: None,
            state: "Elevated".to_string(),
        };

        db.insert_health_rollup_1m(&recent_1m)
            .expect("insert 1m recent");
        db.insert_health_rollup_1m(&expired_1m)
            .expect("insert 1m expired");

        // 2. Insert 1-hour rollups (recent 30 days ago, expired 95 days ago)
        let recent_1h = HealthRollupRecord {
            ts: now_ts - (30 * 86_400),
            cpu_avg: 38.0,
            cpu_max: 70.0,
            mem_avail_pct_min: 40.0,
            swap_in_max: 0.0,
            disk_free_pct_min: 58.0,
            temp_max: 50.0,
            gpu_util_max: None,
            vram_pct_max: None,
            psi_cpu_max: None,
            psi_mem_max: None,
            psi_io_max: None,
            state: "Normal".to_string(),
        };

        let expired_1h = HealthRollupRecord {
            ts: now_ts - (95 * 86_400), // 95 days old (> 90 days)
            cpu_avg: 45.0,
            cpu_max: 75.0,
            mem_avail_pct_min: 30.0,
            swap_in_max: 0.0,
            disk_free_pct_min: 50.0,
            temp_max: 55.0,
            gpu_util_max: None,
            vram_pct_max: None,
            psi_cpu_max: None,
            psi_mem_max: None,
            psi_io_max: None,
            state: "Normal".to_string(),
        };

        db.insert_health_rollup_1h(&recent_1h)
            .expect("insert 1h recent");
        db.insert_health_rollup_1h(&expired_1h)
            .expect("insert 1h expired");

        // 3. Query ranges before pruning
        let list_1m = db
            .get_health_rollups_1m(now_ts - (10 * 86_400), now_ts)
            .expect("query 1m");
        assert_eq!(list_1m.len(), 2);

        let list_1h = db
            .get_health_rollups_1h(now_ts - (100 * 86_400), now_ts)
            .expect("query 1h");
        assert_eq!(list_1h.len(), 2);

        // 4. Run prune job
        let (pruned_1m, pruned_1h) = db.prune_health_rollups(now_ts).expect("prune");
        assert_eq!(pruned_1m, 1, "Exactly 1 expired 1m row should be deleted");
        assert_eq!(pruned_1h, 1, "Exactly 1 expired 1h row should be deleted");

        // 5. Query after prune: only recent rows remain
        let after_1m = db
            .get_health_rollups_1m(now_ts - (10 * 86_400), now_ts)
            .expect("query 1m after prune");
        assert_eq!(after_1m.len(), 1);
        assert_eq!(after_1m[0].ts, recent_1m.ts);

        let after_1h = db
            .get_health_rollups_1h(now_ts - (100 * 86_400), now_ts)
            .expect("query 1h after prune");
        assert_eq!(after_1h.len(), 1);
        assert_eq!(after_1h[0].ts, recent_1h.ts);
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn test_24h_simulated_feed_produces_expected_row_counts() {
        // Verification requirement for 14.5:
        // "Verify: 24 h simulated feed produces expected row counts and prune works."
        let db = Database::open_in_memory().expect("in-memory db");
        let start_ts = 1_700_000_000;

        // 24 hours of 1-minute rollups = 24 * 60 = 1440 entries
        for minute in 0..1440 {
            let ts = start_ts + (minute * 60);
            let r = HealthRollupRecord {
                ts,
                cpu_avg: 20.0 + ((minute % 60) as f64 * 0.5),
                cpu_max: 30.0 + ((minute % 60) as f64 * 0.5),
                mem_avail_pct_min: 45.0,
                swap_in_max: 0.0,
                disk_free_pct_min: 50.0,
                temp_max: 48.0,
                gpu_util_max: None,
                vram_pct_max: None,
                psi_cpu_max: None,
                psi_mem_max: None,
                psi_io_max: None,
                state: "Normal".to_string(),
            };
            db.insert_health_rollup_1m(&r).expect("insert 1m row");
        }

        // 24 hours of 1-hour rollups = 24 entries
        for hour in 0..24 {
            let ts = start_ts + (hour * 3600);
            let r = HealthRollupRecord {
                ts,
                cpu_avg: 35.0,
                cpu_max: 60.0,
                mem_avail_pct_min: 45.0,
                swap_in_max: 0.0,
                disk_free_pct_min: 50.0,
                temp_max: 48.0,
                gpu_util_max: None,
                vram_pct_max: None,
                psi_cpu_max: None,
                psi_mem_max: None,
                psi_io_max: None,
                state: "Normal".to_string(),
            };
            db.insert_health_rollup_1h(&r).expect("insert 1h row");
        }

        // Verify total row counts across 24h
        let day_1m = db
            .get_health_rollups_1m(start_ts, start_ts + (24 * 3600))
            .expect("fetch 24h 1m rollups");
        assert_eq!(
            day_1m.len(),
            1440,
            "24h feed must produce exactly 1440 1-minute rows"
        );

        let day_1h = db
            .get_health_rollups_1h(start_ts, start_ts + (24 * 3600))
            .expect("fetch 24h 1h rollups");
        assert_eq!(
            day_1h.len(),
            24,
            "24h feed must produce exactly 24 1-hour rows"
        );
    }
}
