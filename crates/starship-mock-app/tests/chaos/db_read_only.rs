use super::harness::ChaosMission;
use starship_core::MissionState;

#[test]
fn read_only_database_blocks_mutation_and_preserves_audit_chain() {
    let chaos = ChaosMission::new();
    chaos.start_step();
    chaos.audit("chaos.before_read_only", "success", None);

    chaos
        .db
        .with_conn(|conn| conn.execute_batch("PRAGMA query_only=ON"))
        .expect("enable query_only");
    let error = chaos
        .db
        .update_step_execution(chaos.step_id, "success", Some("{}"), None)
        .expect_err("read-only database must reject write");
    chaos
        .db
        .with_conn(|conn| conn.execute_batch("PRAGMA query_only=OFF"))
        .expect("disable query_only");

    assert_eq!(error.code, "db.query_failed");
    chaos.fail_step(&error.code);
    chaos.assert_final(MissionState::Failed, Some(&error.code));
}
