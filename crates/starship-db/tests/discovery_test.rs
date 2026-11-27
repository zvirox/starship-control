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
