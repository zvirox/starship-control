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
