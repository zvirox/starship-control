use starship_db::{Database, DiscoveredModelRecord};

fn model(id: &str) -> DiscoveredModelRecord {
    DiscoveredModelRecord {
        id: id.to_string(),
        name: format!("Model {id}"),
        provider_type: "cloud_openrouter".to_string(),
        provider_id: "openrouter".to_string(),
        context_window: 32_000,
        supports_tools: true,
        supports_streaming: true,
        input_price: 0.000_001,
        output_price: 0.000_002,
    }
}

#[test]
fn test_discovered_models_upsert_and_missing_marking() {
    let db = Database::open_in_memory().expect("in-memory db");
    assert_eq!(
        db.upsert_discovered_model(&model("router/a"))
            .expect("upsert a"),
        ()
    );
    db.upsert_discovered_model(&model("router/b"))
        .expect("upsert b");

    let models = db.list_models().expect("models");
    let a = models.iter().find(|m| m.id == "router/a").expect("model a");
    assert_eq!(a.context_window, 32_000);
    assert_eq!(a.provider_type, "cloud_openrouter");
    assert_eq!(a.download_status, "remote_only");

    db.set_model_favorite("router/b", true).expect("favorite");
    let marked = db
        .mark_missing_discovered_models("openrouter", &["router/a".to_string()])
        .expect("mark missing");
    assert_eq!(marked, 0, "the only missing model is a favorite");

    db.set_model_favorite("router/b", false)
        .expect("unfavorite");
    let marked = db
        .mark_missing_discovered_models("openrouter", &["router/a".to_string()])
        .expect("mark missing");
    assert_eq!(marked, 1);
    let models = db.list_models().expect("models");
    assert_eq!(
        models
            .iter()
            .find(|m| m.id == "router/b")
            .expect("model b")
            .download_status,
        "remote_only"
    );
}

#[test]
fn test_missing_local_model_is_marked_and_can_be_deleted() {
    let db = Database::open_in_memory().expect("in-memory db");
    db.register_model(
        "local-test",
        "local-test.gguf",
        "local_gguf",
        None,
        Some("/nonexistent/local-test.gguf"),
        Some(1024),
        None,
        Some("abc"),
        2048,
        0,
    )
    .expect("register local model");

    let marked = db.mark_missing_local_models().expect("mark missing");
    assert_eq!(marked, 1);
    let models = db.list_models().expect("models");
    assert_eq!(
        models
            .iter()
            .find(|m| m.id == "local-test")
            .expect("model")
            .download_status,
        "remote_only"
    );

    db.delete_model("local-test").expect("delete model");
    assert!(
        db.list_models()
            .expect("models")
            .iter()
            .all(|m| m.id != "local-test")
    );
}
