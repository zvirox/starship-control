use crate::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use starship_core::executor::{
    MissionEvent, MissionState, StepEvent, StepState, transition_mission, transition_step,
};
use std::collections::HashMap;

/// Database row record from the `mission_events` table (DATABASE.md & GAP-CLOSURE-PLAN.md §11.2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MissionEventRecord {
    pub id: i64,
    pub mission_id: i64,
    pub step_id: Option<i64>,
    pub ts_ms: i64,
    pub kind: String,
    pub payload_json: String,
}

/// New mission event entry to be appended to the append-only `mission_events` log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewMissionEvent {
    pub mission_id: i64,
    pub step_id: Option<i64>,
    pub ts_ms: i64,
    pub kind: String,
    pub payload_json: String,
}

/// Summary of a mission's history, duration, outcome, and fallbacks (GAP-CLOSURE-PLAN.md §13.8).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MissionHistorySummary {
    pub id: i64,
    pub uuid: String,
    pub title: String,
    pub user_goal: String,
    pub status: String,
    pub outcome: String,
    pub duration_ms: i64,
    pub fallback_count: i64,
    pub total_steps: i64,
    pub completed_steps: i64,
    pub active_model_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub peak_cpu_pct: f32,
    pub peak_ram_bytes: u64,
    pub peak_temp_celsius: f32,
    pub governor_interventions: i64,
}

impl Database {
    /// Lists all past and active missions with outcome, duration, and fallback count (GAP-CLOSURE-PLAN.md §13.8).
    pub fn list_mission_history(&self) -> Result<Vec<MissionHistorySummary>, CoreError> {
        let missions = self.list_missions()?;
        let mut summaries = Vec::new();

        for m in missions {
            let events = self.list_mission_events(m.id, None)?;
            let fallback_count =
                i64::try_from(events.iter().filter(|e| e.kind == "fallback").count()).unwrap_or(0);

            let duration_ms = if events.len() >= 2 {
                let first_ts = events.first().map_or(0, |e| e.ts_ms);
                let last_ts = events.last().map_or(0, |e| e.ts_ms);
                (last_ts - first_ts).max(0)
            } else {
                0
            };

            let (total_steps, completed_steps) =
                if let Ok(Some((_plan, steps))) = self.get_active_plan(m.id) {
                    let total = i64::try_from(steps.len()).unwrap_or(0);
                    let completed = i64::try_from(
                        steps
                            .iter()
                            .filter(|s| s.execution_status == "success")
                            .count(),
                    )
                    .unwrap_or(0);
                    (total, completed)
                } else {
                    (0, 0)
                };

            let outcome = match m.status.as_str() {
                "completed" | "completed_final" => {
                    format!("Completed ({completed_steps}/{total_steps} steps)")
                }
                "failed" | "failed_final" => "Failed".to_string(),
                "executing" => format!("Running ({completed_steps}/{total_steps} steps)"),
                other => other.to_string(),
            };

            summaries.push(MissionHistorySummary {
                id: m.id,
                uuid: m.uuid,
                title: m.title,
                user_goal: m.user_goal,
                status: m.status,
                outcome,
                duration_ms,
                fallback_count,
                total_steps,
                completed_steps,
                active_model_id: m.active_model_id,
                created_at: m.created_at,
                updated_at: m.updated_at,
                peak_cpu_pct: m.peak_cpu_pct,
                peak_ram_bytes: m.peak_ram_bytes,
                peak_temp_celsius: m.peak_temp_celsius,
                governor_interventions: m.governor_interventions,
            });
        }

        Ok(summaries)
    }
    /// Appends an event to the append-only `mission_events` table before acting (GAP-CLOSURE-PLAN.md §11.2).
    pub fn record_mission_event(&self, event: &NewMissionEvent) -> Result<i64, CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO mission_events (mission_id, step_id, ts_ms, kind, payload_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    event.mission_id,
                    event.step_id,
                    event.ts_ms,
                    event.kind,
                    event.payload_json,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    /// Fetches all events for a given mission, optionally starting from a cursor sequence ID.
    pub fn list_mission_events(
        &self,
        mission_id: i64,
        since_id: Option<i64>,
    ) -> Result<Vec<MissionEventRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, mission_id, step_id, ts_ms, kind, payload_json
                 FROM mission_events
                 WHERE mission_id = ?1 AND id > ?2
                 ORDER BY id ASC",
            )?;

            let cursor = since_id.unwrap_or(0);
            let rows = stmt.query_map(params![mission_id, cursor], |row| {
                Ok(MissionEventRecord {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    step_id: row.get(2)?,
                    ts_ms: row.get(3)?,
                    kind: row.get(4)?,
                    payload_json: row.get(5)?,
                })
            })?;

            let mut events = Vec::new();
            for r in rows {
                events.push(r?);
            }
            Ok(events)
        })
    }

    /// Rebuilds live mission and step states by sequentially replaying the event log (GAP-CLOSURE-PLAN.md §11.2).
    /// Returns `(MissionState, HashMap<step_id, StepState>)`.
    pub fn rebuild_mission_state(
        &self,
        mission_id: i64,
    ) -> Result<(MissionState, HashMap<i64, StepState>), CoreError> {
        let events = self.list_mission_events(mission_id, None)?;

        let mut mission_state = MissionState::Planning;
        let mut step_states: HashMap<i64, StepState> = HashMap::new();

        for event in events {
            match event.kind.as_str() {
                "mission_transition" => {
                    let mission_ev: MissionEvent = serde_json::from_str(&event.payload_json)
                        .map_err(|e| {
                            CoreError::db(
                                "db.event_parse_error",
                                false,
                                format!("Failed to parse MissionEvent from payload: {e}"),
                                None,
                            )
                        })?;
                    mission_state = transition_mission(mission_state, mission_ev)?;
                }
                "step_transition" => {
                    let step_id = event.step_id.ok_or_else(|| {
                        CoreError::db(
                            "db.event_invalid",
                            false,
                            format!("Step transition event ID {} missing step_id", event.id),
                            None,
                        )
                    })?;

                    let step_ev: StepEvent =
                        serde_json::from_str(&event.payload_json).map_err(|e| {
                            CoreError::db(
                                "db.event_parse_error",
                                false,
                                format!("Failed to parse StepEvent from payload: {e}"),
                                None,
                            )
                        })?;

                    let current_step_state = step_states
                        .get(&step_id)
                        .copied()
                        .unwrap_or(StepState::Pending);
                    let new_step_state = transition_step(current_step_state, step_ev)?;
                    step_states.insert(step_id, new_step_state);
                }
                _ => {
                    // Non-state-machine events (e.g. tool_call, progress, retry telemetry) are preserved in log
                }
            }
        }

        Ok((mission_state, step_states))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontend_reconnect_replays_missed_events_with_no_gaps() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let mission_id = db
            .create_mission(
                "123e4567-e89b-12d3-a456-426614174000",
                "Reconnection Test Mission",
                "Testing cursor-based event replay",
                "qwen2.5-coder-7b-q4",
                "local_gguf",
            )
            .expect("create mission");

        // 1. Record first batch of 5 events
        for i in 1..=5 {
            db.record_mission_event(&NewMissionEvent {
                mission_id,
                step_id: Some(i),
                ts_ms: 1000 + i,
                kind: "progress".to_string(),
                payload_json: format!(r#"{{"stepIndex": {i}, "progress": 0.5}}"#),
            })
            .expect("record event");
        }

        // 2. Initial fetch with no cursor (since_id = None)
        let initial_batch = db
            .list_mission_events(mission_id, None)
            .expect("list events");
        assert_eq!(initial_batch.len(), 5);
        assert_eq!(initial_batch[0].id, 1);
        assert_eq!(initial_batch[4].id, 5);

        // Client saves cursor = 5
        let last_cursor = initial_batch.last().unwrap().id;
        assert_eq!(last_cursor, 5);

        // 3. Client disconnects. Host records 5 more events while client is offline (events 6..=10)
        for i in 6..=10 {
            db.record_mission_event(&NewMissionEvent {
                mission_id,
                step_id: Some(i),
                ts_ms: 2000 + i,
                kind: "progress".to_string(),
                payload_json: format!(r#"{{"stepIndex": {i}, "progress": 1.0}}"#),
            })
            .expect("record event");
        }

        // 4. Client reconnects and requests events since cursor = 5
        let replayed_batch = db
            .list_mission_events(mission_id, Some(last_cursor))
            .expect("list events since cursor");

        // Verification requirement for 13.4:
        // "frontend reconnect replays missed events with no gaps (cursor test)"
        assert_eq!(replayed_batch.len(), 5);
        let replayed_ids: Vec<i64> = replayed_batch.iter().map(|e| e.id).collect();
        assert_eq!(replayed_ids, vec![6, 7, 8, 9, 10]);

        // Verify full continuity: combining initial + replayed batch produces unbroken contiguous sequence [1..=10]
        let mut full_sequence = initial_batch;
        full_sequence.extend(replayed_batch);
        assert_eq!(full_sequence.len(), 10);
        for (idx, evt) in full_sequence.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            let expected_id = (idx + 1) as i64;
            assert_eq!(evt.id, expected_id);
        }
    }

    #[test]
    fn test_replay_of_finished_mission_reproduces_final_state_exactly() {
        let db = Database::open_in_memory().expect("open in-memory db");
        let mission_id = db
            .create_mission(
                "333e4567-e89b-12d3-a456-426614174333",
                "Replay Test Mission",
                "Executing record, denoise, and cut",
                "qwen2.5-coder-7b-q4",
                "local_gguf",
            )
            .expect("create mission");

        // 1. Mission generates plan and starts execution
        db.record_mission_event(&NewMissionEvent {
            mission_id,
            step_id: None,
            ts_ms: 1000,
            kind: "mission_transition".to_string(),
            payload_json: serde_json::to_string(&MissionEvent::PlanGenerated).unwrap(),
        })
        .expect("record plan generated");

        db.record_mission_event(&NewMissionEvent {
            mission_id,
            step_id: None,
            ts_ms: 1050,
            kind: "mission_transition".to_string(),
            payload_json: serde_json::to_string(&MissionEvent::Start).unwrap(),
        })
        .expect("record mission start");

        // 2. Step 1 (Safe Tier 1): Scheduled -> Running -> Succeeded
        let step_1_events = [
            StepEvent::Schedule,
            StepEvent::DispatchDirect,
            StepEvent::Succeed,
        ];
        for (i, ev) in step_1_events.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            let ts_ms = 1100 + (i as i64 * 50);
            db.record_mission_event(&NewMissionEvent {
                mission_id,
                step_id: Some(1),
                ts_ms,
                kind: "step_transition".to_string(),
                payload_json: serde_json::to_string(ev).unwrap(),
            })
            .expect("record step 1 event");
        }

        // 3. Step 2 (Mutating Tier 2): Scheduled -> RequireApproval -> ApproveAndDispatch -> Succeed
        let step_2_events = [
            StepEvent::Schedule,
            StepEvent::RequireApproval,
            StepEvent::ApproveAndDispatch,
            StepEvent::Succeed,
        ];
        for (i, ev) in step_2_events.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            let ts_ms = 1400 + (i as i64 * 50);
            db.record_mission_event(&NewMissionEvent {
                mission_id,
                step_id: Some(2),
                ts_ms,
                kind: "step_transition".to_string(),
                payload_json: serde_json::to_string(ev).unwrap(),
            })
            .expect("record step 2 event");
        }

        // 4. Mission finishes with all steps succeeded
        db.record_mission_event(&NewMissionEvent {
            mission_id,
            step_id: None,
            ts_ms: 2000,
            kind: "mission_transition".to_string(),
            payload_json: serde_json::to_string(&MissionEvent::CompleteAllSucceeded).unwrap(),
        })
        .expect("record mission completed");

        // GAP-CLOSURE-PLAN.md §13.8 Requirement:
        // "Verify: replay of a finished mission reproduces final state exactly."
        let (replayed_mission_state, replayed_step_states) = db
            .rebuild_mission_state(mission_id)
            .expect("rebuild mission state from event log");

        assert_eq!(
            replayed_mission_state,
            MissionState::Succeeded,
            "Replayed mission state must match Succeeded exactly"
        );

        assert_eq!(
            replayed_step_states.get(&1),
            Some(&StepState::Succeeded),
            "Step 1 replayed state must be Succeeded"
        );

        assert_eq!(
            replayed_step_states.get(&2),
            Some(&StepState::Succeeded),
            "Step 2 replayed state must be Succeeded"
        );

        // Verify history listing
        let history = db.list_mission_history().expect("list mission history");
        assert_eq!(history.len(), 1);
        let summary = &history[0];
        assert_eq!(summary.id, mission_id);
        assert_eq!(summary.duration_ms, 1000); // 2000 - 1000 = 1000ms
        assert_eq!(summary.fallback_count, 0);
    }

    #[test]
    fn test_completed_mock_mission_has_populated_health_summary() {
        // Verification for GAP-CLOSURE-PLAN.md §14.11:
        // "Verify: completed mock mission has populated summary."
        let db = Database::open_in_memory().expect("in-memory db");
        let mission_id = db
            .create_mission(
                "ms-test-14-11",
                "Health Summary Test Mission",
                "Execute video transcode with load telemetry",
                "qwen2.5-coder-7b-q4",
                "local_gguf",
            )
            .expect("create mission");

        // Record health samples across execution
        db.record_mission_health_sample(mission_id, 45.0, 4 * 1024 * 1024 * 1024, 52.0, false)
            .expect("record health sample 1");

        // Peak load event with governor intervention
        db.record_mission_health_sample(mission_id, 88.5, 9 * 1024 * 1024 * 1024, 76.2, true)
            .expect("record health sample 2 (peak + governor intervention)");

        // Later quiet sample (peaks should be retained by MAX)
        db.record_mission_health_sample(mission_id, 30.0, 3 * 1024 * 1024 * 1024, 48.0, false)
            .expect("record health sample 3");

        // Fetch mission record directly
        let mission = db
            .get_mission(mission_id)
            .expect("get mission")
            .expect("mission exists");
        assert_eq!(mission.peak_cpu_pct, 88.5);
        assert_eq!(mission.peak_ram_bytes, 9 * 1024 * 1024 * 1024);
        assert_eq!(mission.peak_temp_celsius, 76.2);
        assert_eq!(mission.governor_interventions, 1);

        // Fetch mission history summary
        let history = db.list_mission_history().expect("list mission history");
        let summary = history
            .iter()
            .find(|h| h.id == mission_id)
            .expect("history record");
        assert_eq!(summary.peak_cpu_pct, 88.5);
        assert_eq!(summary.peak_ram_bytes, 9 * 1024 * 1024 * 1024);
        assert_eq!(summary.peak_temp_celsius, 76.2);
        assert_eq!(summary.governor_interventions, 1);
    }
}
