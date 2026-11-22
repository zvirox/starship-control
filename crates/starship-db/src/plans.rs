use crate::Database;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use starship_core::planner::TaskSheet;

/// Database row record from the `plans` table (DATABASE.md §3.3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanRecord {
    pub id: i64,
    pub mission_id: i64,
    pub version: i64,
    pub title: String,
    pub summary: String,
    pub raw_tasksheet_json: String,
    pub status: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Database row record from the `plan_steps` table (DATABASE.md §3.4).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanStepRecord {
    pub id: i64,
    pub plan_id: i64,
    pub step_index: i64,
    pub app_id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub risk_tier: i64,
    pub approval_status: String,
    pub execution_status: String,
    pub diff_summary: Option<String>,
    pub result_json: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

impl Database {
    /// Atomically persists a new plan and all its discrete steps.
    pub fn create_plan(
        &self,
        mission_id: i64,
        title: &str,
        summary: &str,
        tasksheet: &TaskSheet,
    ) -> Result<PlanRecord, CoreError> {
        let tasksheet_json = tasksheet.to_json()?;
        let initial_status = if tasksheet.requires_approval() {
            "pending_approval"
        } else {
            "executing"
        };

        self.with_conn(|conn| {
            let tx = conn.transaction()?;

            tx.execute(
                "INSERT INTO plans (mission_id, version, title, summary, raw_tasksheet_json, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    mission_id,
                    i64::from(tasksheet.version),
                    title,
                    summary,
                    tasksheet_json,
                    initial_status,
                ],
            )?;

            let plan_id = tx.last_insert_rowid();

            for step in &tasksheet.steps {
                let args_json =
                    serde_json::to_string(&step.arguments).unwrap_or_else(|_| "{}".to_string());
                let approval_status = if step.risk_tier == 1 {
                    "auto"
                } else {
                    "pending"
                };

                tx.execute(
                    "INSERT INTO plan_steps (
                        plan_id, step_index, app_id, tool_name, arguments_json,
                        risk_tier, approval_status, execution_status, diff_summary
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)",
                    params![
                        plan_id,
                        i64::from(step.step_index),
                        step.app_id,
                        step.tool_name,
                        args_json,
                        i64::from(step.risk_tier),
                        approval_status,
                        step.diff_summary,
                    ],
                )?;
            }

            let mut stmt = tx.prepare(
                "SELECT id, mission_id, version, title, summary, raw_tasksheet_json,
                        status, approved_by, approved_at, created_at, updated_at
                 FROM plans WHERE id = ?1",
            )?;

            let record = stmt.query_row(params![plan_id], |row| {
                Ok(PlanRecord {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    version: row.get(2)?,
                    title: row.get(3)?,
                    summary: row.get(4)?,
                    raw_tasksheet_json: row.get(5)?,
                    status: row.get(6)?,
                    approved_by: row.get(7)?,
                    approved_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?;

            drop(stmt);
            tx.commit()?;
            Ok(record)
        })
    }

    /// Fetches the most recent active or pending plan for a mission with all its steps.
    pub fn get_active_plan(
        &self,
        mission_id: i64,
    ) -> Result<Option<(PlanRecord, Vec<PlanStepRecord>)>, CoreError> {
        self.with_conn(|conn| {
            let mut plan_stmt = conn.prepare(
                "SELECT id, mission_id, version, title, summary, raw_tasksheet_json,
                        status, approved_by, approved_at, created_at, updated_at
                 FROM plans
                 WHERE mission_id = ?1
                 ORDER BY id DESC LIMIT 1",
            )?;

            let plan_opt = plan_stmt
                .query_row(params![mission_id], |row| {
                    Ok(PlanRecord {
                        id: row.get(0)?,
                        mission_id: row.get(1)?,
                        version: row.get(2)?,
                        title: row.get(3)?,
                        summary: row.get(4)?,
                        raw_tasksheet_json: row.get(5)?,
                        status: row.get(6)?,
                        approved_by: row.get(7)?,
                        approved_at: row.get(8)?,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    })
                })
                .optional()?;

            let Some(plan) = plan_opt else {
                return Ok(None);
            };

            let mut steps_stmt = conn.prepare(
                "SELECT id, plan_id, step_index, app_id, tool_name, arguments_json,
                        risk_tier, approval_status, execution_status, diff_summary,
                        result_json, error_message, started_at, finished_at
                 FROM plan_steps
                 WHERE plan_id = ?1
                 ORDER BY step_index ASC",
            )?;

            let step_rows = steps_stmt.query_map(params![plan.id], |row| {
                Ok(PlanStepRecord {
                    id: row.get(0)?,
                    plan_id: row.get(1)?,
                    step_index: row.get(2)?,
                    app_id: row.get(3)?,
                    tool_name: row.get(4)?,
                    arguments_json: row.get(5)?,
                    risk_tier: row.get(6)?,
                    approval_status: row.get(7)?,
                    execution_status: row.get(8)?,
                    diff_summary: row.get(9)?,
                    result_json: row.get(10)?,
                    error_message: row.get(11)?,
                    started_at: row.get(12)?,
                    finished_at: row.get(13)?,
                })
            })?;

            let mut steps = Vec::new();
            for step in step_rows {
                steps.push(step?);
            }

            Ok(Some((plan, steps)))
        })
    }

    /// Fetches a single step by ID.
    pub fn get_plan_step(&self, step_id: i64) -> Result<Option<PlanStepRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, plan_id, step_index, app_id, tool_name, arguments_json,
                        risk_tier, approval_status, execution_status, diff_summary,
                        result_json, error_message, started_at, finished_at
                 FROM plan_steps
                 WHERE id = ?1",
            )?;

            stmt.query_row(params![step_id], |row| {
                Ok(PlanStepRecord {
                    id: row.get(0)?,
                    plan_id: row.get(1)?,
                    step_index: row.get(2)?,
                    app_id: row.get(3)?,
                    tool_name: row.get(4)?,
                    arguments_json: row.get(5)?,
                    risk_tier: row.get(6)?,
                    approval_status: row.get(7)?,
                    execution_status: row.get(8)?,
                    diff_summary: row.get(9)?,
                    result_json: row.get(10)?,
                    error_message: row.get(11)?,
                    started_at: row.get(12)?,
                    finished_at: row.get(13)?,
                })
            })
            .optional()
        })
    }

    /// Updates approval status for a specific plan step (`'approved'` or `'rejected'`).
    pub fn update_step_approval(&self, step_id: i64, status: &str) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE plan_steps
                 SET approval_status = ?1
                 WHERE id = ?2",
                params![status, step_id],
            )?;
            Ok(())
        })
    }

    /// Updates execution status and output for a specific plan step.
    pub fn update_step_execution(
        &self,
        step_id: i64,
        execution_status: &str,
        result_json: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE plan_steps
                 SET execution_status = ?1,
                     result_json = ?2,
                     error_message = ?3,
                     finished_at = CASE WHEN ?1 IN ('success', 'failed', 'skipped') THEN strftime('%Y-%m-%dT%H:%M:%SZ', 'now') ELSE finished_at END,
                     started_at = CASE WHEN ?1 = 'running' AND started_at IS NULL THEN strftime('%Y-%m-%dT%H:%M:%SZ', 'now') ELSE started_at END
                 WHERE id = ?4",
                params![execution_status, result_json, error_message, step_id],
            )?;
            Ok(())
        })
    }

    /// Updates the overall status of a plan.
    pub fn update_plan_status(
        &self,
        plan_id: i64,
        status: &str,
        approved_by: Option<&str>,
    ) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE plans
                 SET status = ?1,
                     approved_by = ?2,
                     approved_at = CASE WHEN ?2 IS NOT NULL THEN strftime('%Y-%m-%dT%H:%M:%SZ', 'now') ELSE approved_at END,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?3",
                params![status, approved_by, plan_id],
            )?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::NewAuditEntry;
    use serde_json::json;
    use starship_core::planner::TaskSheetStep;

    #[test]
    fn test_plan_lifecycle_and_step_execution() {
        let db = Database::open_in_memory().expect("in-memory db");

        // First create a parent mission
        let mission_id = db
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO missions (uuid, title, user_goal, active_model_id, provider_type)
                     VALUES ('test-uuid-42', 'Test Mission', 'Record screen and clean audio', 'qwen2.5-coder-7b-q4', 'local_gguf')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })
            .expect("insert mission");

        let steps = vec![
            TaskSheetStep {
                step_index: 0,
                app_id: "ssp".to_string(),
                tool_name: "recording.start".to_string(),
                arguments: json!({ "target": "screen" }),
                risk_tier: 1,
                diff_summary: "Starts screen capture".to_string(),
                depends_on: vec![],
            },
            TaskSheetStep {
                step_index: 1,
                app_id: "alp".to_string(),
                tool_name: "audio.cleanup".to_string(),
                arguments: json!({ "preset": "voice_isolate" }),
                risk_tier: 2,
                diff_summary: "Applies vocal cleanup".to_string(),
                depends_on: vec![0],
            },
        ];

        let sheet = TaskSheet::new(
            "Record screen and clean audio",
            "Screen Recording & Cleanup",
            "Records and isolates voice",
            steps,
        );

        let plan = db
            .create_plan(mission_id, &sheet.title, &sheet.summary, &sheet)
            .expect("create plan");

        assert_eq!(plan.mission_id, mission_id);
        assert_eq!(plan.status, "pending_approval");

        let active = db.get_active_plan(mission_id).expect("get active plan");
        assert!(active.is_some());
        let (active_plan, active_steps) = active.unwrap();
        assert_eq!(active_plan.id, plan.id);
        assert_eq!(active_steps.len(), 2);
        assert_eq!(active_steps[0].approval_status, "auto"); // tier 1 is auto
        assert_eq!(active_steps[1].approval_status, "pending"); // tier 2 is pending

        // Update step approval
        let step_2_id = active_steps[1].id;
        db.update_step_approval(step_2_id, "approved")
            .expect("update approval");
        let updated_step = db.get_plan_step(step_2_id).expect("get step").unwrap();
        assert_eq!(updated_step.approval_status, "approved");

        // Update step execution
        db.update_step_execution(step_2_id, "success", Some("{\"cleaned\": true}"), None)
            .expect("update execution");
        let finished_step = db.get_plan_step(step_2_id).expect("get step").unwrap();
        assert_eq!(finished_step.execution_status, "success");
        assert_eq!(
            finished_step.result_json.as_deref(),
            Some("{\"cleaned\": true}")
        );

        // Record signed audit log
        let audit_entry = NewAuditEntry {
            mission_id: Some(mission_id),
            app_id: "alp".to_string(),
            action_type: "approval_granted".to_string(),
            risk_tier: 2,
            payload_json: "{\"stepId\": 101, \"toolName\": \"audio.cleanup\"}".to_string(),
            status: "success".to_string(),
            signature_token: Some("usr_sig_test123456789".to_string()),
        };

        let audit_id = db.record_audit_log(&audit_entry).expect("record audit log");
        assert!(audit_id > 0);

        let logs = db.list_audit_logs(10, 0).expect("list audit logs");
        assert_eq!(logs.len(), 1);
        assert_eq!(
            logs[0].signature_token.as_deref(),
            Some("usr_sig_test123456789")
        );
        assert_eq!(logs[0].status, "success");
    }
}
