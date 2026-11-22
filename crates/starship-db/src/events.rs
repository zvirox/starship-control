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

impl Database {
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
}
