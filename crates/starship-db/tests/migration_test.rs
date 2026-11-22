use rusqlite::params;
use starship_db::Database;
use tempfile::tempdir;

#[test]
#[allow(clippy::assert_is_empty)]
fn test_database_initial_migration_and_seed_data() {
    let db = Database::open_in_memory().expect("in-memory db open");

    // Verify seed models exist
    let models = db.list_models().expect("list models");
    assert!(!models.is_empty(), "Expected seed models to be inserted");
    let default_model = models.iter().find(|m| m.is_default);
    assert!(default_model.is_some(), "Expected default model to exist");
    assert_eq!(default_model.unwrap().id, "qwen2.5-coder-7b-q4");

    // Verify seed provider configs exist
    let provider_count: i64 = db
        .with_conn(|conn| {
            conn.query_row("SELECT count(*) FROM provider_configs", [], |row| {
                row.get(0)
            })
        })
        .expect("count provider_configs");
    assert_eq!(provider_count, 4, "Expected 4 seed providers");
}

#[test]
fn test_mission_lifecycle_and_cascading_deletes() {
    let db = Database::open_in_memory().expect("in-memory db open");

    // Create a mission
    let mission_id = db
        .create_mission(
            "123e4567-e89b-12d3-a456-426614174000",
            "Test Screencast Assembly",
            "Trim silences and mix voiceover",
            "qwen2.5-coder-7b-q4",
            "local_gguf",
        )
        .expect("create mission");

    assert!(mission_id > 0);

    let mission = db
        .get_mission(mission_id)
        .expect("get mission")
        .expect("mission found");
    assert_eq!(mission.title, "Test Screencast Assembly");
    assert_eq!(mission.status, "active");

    // Insert a message into mission_messages
    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO mission_messages (mission_id, role, content) VALUES (?1, ?2, ?3)",
            params![mission_id, "user", "Start workflow"],
        )?;
        Ok(())
    })
    .expect("insert message");

    // Insert a plan and a step
    let plan_id: i64 = db
        .with_conn(|conn| {
            conn.execute(
                "INSERT INTO plans (mission_id, version, title, summary, raw_tasksheet_json)
                 VALUES (?1, 1, 'Trim Silence', 'Trims dead air in EEP', '{}')",
                params![mission_id],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .expect("insert plan");

    db.with_conn(|conn| {
        conn.execute(
            "INSERT INTO plan_steps (plan_id, step_index, app_id, tool_name, arguments_json, risk_tier)
             VALUES (?1, 0, 'eep', 'timeline.cut_interval', '{}', 2)",
            params![plan_id],
        )?;
        Ok(())
    })
    .expect("insert step");

    // Verify messages and plan steps exist
    let msg_count: i64 = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT count(*) FROM mission_messages WHERE mission_id = ?1",
                params![mission_id],
                |r| r.get(0),
            )
        })
        .expect("query messages");
    assert_eq!(msg_count, 1);

    // Delete mission and verify cascading deletion
    db.with_conn(|conn| {
        conn.execute("DELETE FROM missions WHERE id = ?1", params![mission_id])?;
        Ok(())
    })
    .expect("delete mission");

    let post_msg_count: i64 = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT count(*) FROM mission_messages WHERE mission_id = ?1",
                params![mission_id],
                |r| r.get(0),
            )
        })
        .expect("query messages post-delete");
    assert_eq!(
        post_msg_count, 0,
        "Cascading delete should clear mission_messages"
    );

    let post_plan_count: i64 = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT count(*) FROM plans WHERE mission_id = ?1",
                params![mission_id],
                |r| r.get(0),
            )
        })
        .expect("query plans post-delete");
    assert_eq!(
        post_plan_count, 0,
        "Cascading delete should clear plans and steps"
    );
}

#[test]
#[allow(clippy::assert_is_empty)]
fn test_file_backed_database_permissions() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("library.db");

    let db = Database::open_path(&db_path).expect("open file db");
    assert!(db.path().is_some());
    assert!(db_path.exists());

    let models = db.list_models().expect("list models from file db");
    assert!(!models.is_empty(), "Expected models from file db");
}

#[test]
fn test_secrets_at_rest_encrypted_in_sqlite() {
    // GAP-CLOSURE-PLAN.md §10.7: Canary key never appears in DB plaintext
    let db = Database::open_in_memory().expect("in-memory db open");
    let canary_key = "sk-proj-canary-never-plain-in-sqlite-998877665544332211";

    db.set_provider_config(
        "openai",
        "OpenAI Cloud",
        "https://api.openai.com/v1",
        Some(canary_key),
        true,
    )
    .expect("set provider config");

    // Directly query raw SQLite row to ensure ciphertext is stored, NOT plaintext
    let stored_raw: String = db
        .with_conn(|conn| {
            conn.query_row(
                "SELECT api_key_encrypted FROM provider_configs WHERE provider_id = 'openai'",
                [],
                |row| row.get(0),
            )
        })
        .expect("query raw api_key_encrypted");

    assert!(stored_raw.starts_with("enc_v1:"));
    assert!(
        !stored_raw.contains(canary_key),
        "CRITICAL: Canary API key found in raw SQLite plaintext!"
    );

    // Decrypting recovers original canary key
    let decrypted = Database::decrypt_api_key(&stored_raw).expect("decrypt api key");
    assert_eq!(decrypted, canary_key);
}

#[test]
fn test_tamper_evident_audit_log_hash_chain_and_triggers() {
    // GAP-CLOSURE-PLAN.md §10.8: Tamper-evident hash chained audit log and SQLite triggers
    use starship_db::audit::NewAuditEntry;

    let db = Database::open_in_memory().expect("in-memory db open");

    let mission_id = db
        .create_mission(
            "123e4567-e89b-12d3-a456-426614174099",
            "Audit Chain Mission",
            "Mission for verifying audit logs",
            "qwen2.5-coder-7b-q4",
            "local_gguf",
        )
        .expect("create mission");

    // 1. Insert multiple audit records
    let id1 = db
        .record_audit_log(&NewAuditEntry {
            mission_id: Some(mission_id),
            app_id: "eep".into(),
            action_type: "timeline.cut".into(),
            risk_tier: 2,
            payload_json: r#"{"startMs":0,"endMs":5000}"#.into(),
            status: "approved".into(),
            signature_token: Some("sig-token-1".into()),
        })
        .expect("record entry 1");

    let id2 = db
        .record_audit_log(&NewAuditEntry {
            mission_id: Some(mission_id),
            app_id: "alp".into(),
            action_type: "audio.denoise".into(),
            risk_tier: 1,
            payload_json: r#"{"threshold":-18}"#.into(),
            status: "executed".into(),
            signature_token: None,
        })
        .expect("record entry 2");

    let id3 = db
        .record_audit_log(&NewAuditEntry {
            mission_id: None,
            app_id: "ssp".into(),
            action_type: "screen.record".into(),
            risk_tier: 2,
            payload_json: r#"{"display":0}"#.into(),
            status: "completed".into(),
            signature_token: Some("sig-token-3".into()),
        })
        .expect("record entry 3");

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);

    // 2. Verify untampered chain
    let count = db.verify_audit_chain().expect("verify audit chain");
    assert_eq!(count, 3, "Expected 3 verified audit records in chain");

    // 3. Test that SQLite triggers prevent direct UPDATE
    let update_res = db.with_conn(|conn| {
        conn.execute("UPDATE audit_log SET status = 'tampered' WHERE id = 2", [])
    });
    assert!(
        update_res.is_err(),
        "Expected SQLite trigger to prohibit UPDATE on audit_log"
    );
    let update_err = update_res.unwrap_err().to_string();
    assert!(
        update_err.contains("UPDATE prohibited"),
        "Expected 'UPDATE prohibited' trigger error message, got: {update_err}"
    );

    // 4. Test that SQLite triggers prevent direct DELETE
    let delete_res = db.with_conn(|conn| conn.execute("DELETE FROM audit_log WHERE id = 3", []));
    assert!(
        delete_res.is_err(),
        "Expected SQLite trigger to prohibit DELETE on audit_log"
    );
    let delete_err = delete_res.unwrap_err().to_string();
    assert!(
        delete_err.contains("DELETE prohibited"),
        "Expected 'DELETE prohibited' trigger error message, got: {delete_err}"
    );

    // 5. Simulate unauthorized bypass/tampering (e.g. dropping trigger and mutating row 2)
    db.with_conn(|conn| {
        conn.execute("DROP TRIGGER prevent_audit_log_update", [])?;
        conn.execute(
            "UPDATE audit_log SET payload_json = '{\"tampered\":true}' WHERE id = 2",
            [],
        )?;
        Ok(())
    })
    .expect("simulate tampering");

    // 6. Verify audit chain detects tampering and names the exact row (row 2)
    let tamper_res = db.verify_audit_chain();
    assert!(
        tamper_res.is_err(),
        "Expected verify_audit_chain to detect tampering"
    );
    let err = tamper_res.unwrap_err();
    assert_eq!(err.code, "db.audit_tampered");
    assert!(
        err.user_message.contains("record ID 2"),
        "Expected error to pinpoint record ID 2, got: {err:?}"
    );
}

#[test]
fn test_event_sourced_run_log_and_rebuild_mission_state() {
    // GAP-CLOSURE-PLAN.md §11.2: Event-sourced run log & state reconstruction
    use starship_core::executor::{MissionEvent, MissionState, PauseReason, StepEvent, StepState};
    use starship_db::events::NewMissionEvent;

    let db = Database::open_in_memory().expect("in-memory db open");

    let mission_id = db
        .create_mission(
            "987e6543-e21b-12d3-a456-426614174000",
            "Event Sourcing Test Mission",
            "Testing append-only run log and replay",
            "qwen2.5-coder-7b-q4",
            "local_gguf",
        )
        .expect("create mission");

    let step_1 = 101_i64;
    let step_2 = 102_i64;

    // 1. Initial State before events: Planning, no step records
    let (initial_mission, initial_steps) = db
        .rebuild_mission_state(mission_id)
        .expect("rebuild initial");
    assert_eq!(initial_mission, MissionState::Planning);
    assert!(initial_steps.is_empty());

    // 2. Mission transitions Planning -> Ready -> Running
    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: None,
        ts_ms: 1000,
        kind: "mission_transition".into(),
        payload_json: serde_json::to_string(&MissionEvent::PlanGenerated).unwrap(),
    })
    .expect("record event 1");

    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: None,
        ts_ms: 1005,
        kind: "mission_transition".into(),
        payload_json: serde_json::to_string(&MissionEvent::Start).unwrap(),
    })
    .expect("record event 2");

    // 3. Step 101 transitions: Schedule -> DispatchDirect -> Succeed
    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: Some(step_1),
        ts_ms: 1010,
        kind: "step_transition".into(),
        payload_json: serde_json::to_string(&StepEvent::Schedule).unwrap(),
    })
    .expect("record event 3");

    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: Some(step_1),
        ts_ms: 1015,
        kind: "step_transition".into(),
        payload_json: serde_json::to_string(&StepEvent::DispatchDirect).unwrap(),
    })
    .expect("record event 4");

    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: Some(step_1),
        ts_ms: 1020,
        kind: "step_transition".into(),
        payload_json: serde_json::to_string(&StepEvent::Succeed).unwrap(),
    })
    .expect("record event 5");

    // 4. Step 102 transitions: Schedule -> RequireApproval -> Mission pauses on Approval
    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: Some(step_2),
        ts_ms: 1025,
        kind: "step_transition".into(),
        payload_json: serde_json::to_string(&StepEvent::Schedule).unwrap(),
    })
    .expect("record event 6");

    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: Some(step_2),
        ts_ms: 1030,
        kind: "step_transition".into(),
        payload_json: serde_json::to_string(&StepEvent::RequireApproval).unwrap(),
    })
    .expect("record event 7");

    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: None,
        ts_ms: 1035,
        kind: "mission_transition".into(),
        payload_json: serde_json::to_string(&MissionEvent::Pause(PauseReason::Approval)).unwrap(),
    })
    .expect("record event 8");

    // 5. Rebuild mid-execution state from events
    let (mid_mission, mid_steps) = db.rebuild_mission_state(mission_id).expect("rebuild mid");
    assert_eq!(mid_mission, MissionState::Paused(PauseReason::Approval));
    assert_eq!(mid_steps.get(&step_1), Some(&StepState::Succeeded));
    assert_eq!(mid_steps.get(&step_2), Some(&StepState::AwaitingApproval));

    // 6. Test cursor pagination for event streaming resync (cmd://mission_get_timeline)
    let events_after_4 = db
        .list_mission_events(mission_id, Some(4))
        .expect("cursor list");
    assert_eq!(events_after_4.len(), 4); // events 5, 6, 7, 8
    assert_eq!(events_after_4[0].id, 5);

    // 7. Human approves Step 102: ApproveAndDispatch -> Mission Resume -> Succeed -> Complete
    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: Some(step_2),
        ts_ms: 1040,
        kind: "step_transition".into(),
        payload_json: serde_json::to_string(&StepEvent::ApproveAndDispatch).unwrap(),
    })
    .expect("record event 9");

    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: None,
        ts_ms: 1045,
        kind: "mission_transition".into(),
        payload_json: serde_json::to_string(&MissionEvent::Resume).unwrap(),
    })
    .expect("record event 10");

    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: Some(step_2),
        ts_ms: 1050,
        kind: "step_transition".into(),
        payload_json: serde_json::to_string(&StepEvent::Succeed).unwrap(),
    })
    .expect("record event 11");

    db.record_mission_event(&NewMissionEvent {
        mission_id,
        step_id: None,
        ts_ms: 1055,
        kind: "mission_transition".into(),
        payload_json: serde_json::to_string(&MissionEvent::CompleteAllSucceeded).unwrap(),
    })
    .expect("record event 12");

    // 8. Final replay rebuilds completed mission and steps with 100% fidelity
    let (final_mission, final_steps) = db.rebuild_mission_state(mission_id).expect("rebuild final");
    assert_eq!(final_mission, MissionState::Succeeded);
    assert_eq!(final_steps.get(&step_1), Some(&StepState::Succeeded));
    assert_eq!(final_steps.get(&step_2), Some(&StepState::Succeeded));
}
