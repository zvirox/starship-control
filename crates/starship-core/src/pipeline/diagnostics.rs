use crate::planner::tasksheet::{TaskSheet, TaskSheetStep};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Telemetry metrics aggregated from NWP and ATP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsReport {
    pub live_connections_count: usize,
    pub total_bandwidth_kbps: f64,
    pub disk_used_gb: f64,
    pub disk_free_gb: f64,
    pub cache_bloat_mb: f64,
    pub reclaimable_items: Vec<String>,
}

/// Pipeline orchestrator for system health diagnostics (NWP + ATP).
pub struct DiagnosticsPipeline;

impl DiagnosticsPipeline {
    /// Generates a diagnostic and cleanup task sheet with a Tier 3 approval gate.
    #[must_use]
    pub fn plan_system_maintenance(metrics: &SystemMetricsReport) -> TaskSheet {
        let bloat_gb = metrics.cache_bloat_mb / 1024.0;
        let diff_summary = format!(
            "Purges {bloat_gb:.1} GB of obsolete build artifacts, package caches, and temporary files"
        );

        let steps = vec![
            // Step 0: Scan live network sockets and suspicious traffic via NWP
            TaskSheetStep {
                step_index: 0,
                app_id: "nwp".to_string(),
                tool_name: "network.scan".to_string(),
                arguments: json!({
                    "inspectSockets": true,
                    "resolveGeoIp": false,
                }),
                risk_tier: 1,
                diff_summary: format!(
                    "Inspects {} active network sockets and bandwidth telemetry",
                    metrics.live_connections_count
                ),
                depends_on: vec![],
            },
            // Step 1: Scan storage breakdown and directory bloat via ATP
            TaskSheetStep {
                step_index: 1,
                app_id: "atp".to_string(),
                tool_name: "disk.scan".to_string(),
                arguments: json!({
                    "targets": ["package_cache", "build_artifacts", "temp_files"],
                }),
                risk_tier: 1,
                diff_summary: format!(
                    "Scans disk utilization ({:.1} GB free) and identifies obsolete caches",
                    metrics.disk_free_gb
                ),
                depends_on: vec![],
            },
            // Step 2: Purge caches via ATP (Tier 3 Destructive Approval Gate)
            TaskSheetStep {
                step_index: 2,
                app_id: "atp".to_string(),
                tool_name: "cache.purge_all".to_string(),
                arguments: json!({
                    "olderThanDays": 7,
                    "targets": metrics.reclaimable_items,
                    "createRestorePoint": true,
                }),
                risk_tier: 3,
                diff_summary,
                depends_on: vec![0, 1],
            },
        ];

        TaskSheet::new(
            "System Health Inspection & Cache Cleanup",
            "System Maintenance & Cache Optimization",
            format!(
                "Inspects live network sockets, analyzes disk usage, and cleans {bloat_gb:.1} GB of temporary build caches upon confirmation"
            ),
            steps,
        )
    }
}
