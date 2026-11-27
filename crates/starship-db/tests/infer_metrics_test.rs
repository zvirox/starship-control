use starship_db::{Database, InferMetricRecord};

#[test]
fn test_infer_metrics_record_and_history_are_real_and_empty_first() {
    let db = Database::open_in_memory().expect("in-memory db");
    let version: u32 = db
        .with_conn(|conn| conn.query_row("PRAGMA user_version", [], |row| row.get(0)))
        .expect("read schema version");
    assert_eq!(version, 7, "migration 0007 must be applied");
    assert!(
        db.list_infer_metrics("15m")
            .expect("empty history")
            .is_empty(),
        "a fresh profile must have no fabricated telemetry"
    );

    let metric = InferMetricRecord {
        id: 0,
        ts_ms: 0, // the DB assigns the real timestamp at insert time
        mission_id: Some(42),
        provider: "cloud_openrouter".to_string(),
        model_id: "openrouter/qwen".to_string(),
        first_token_ms: 350,
        total_latency_ms: 2100,
        tokens_in: 0,
        tokens_out: 17,
        tokens_per_sec: 8.095,
        error_code: None,
    };
    db.record_infer_metric(&metric).expect("record metric");

    let rows = db.list_infer_metrics("15m").expect("history");
    assert_eq!(rows.len(), 1);
    assert!(rows[0].ts_ms > 0, "timestamp must be measured, not zero");
    assert_eq!(rows[0].model_id, metric.model_id);
    assert_eq!(rows[0].first_token_ms, 350);
    assert_eq!(rows[0].tokens_out, 17);
    assert!((rows[0].tokens_per_sec - 8.095).abs() < f64::EPSILON);
}
