use crate::Database;
use rusqlite::params;
use starship_core::CoreError;
use std::time::{SystemTime, UNIX_EPOCH};

/// A model returned by a provider `/models` endpoint, persisted locally.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredModelRecord {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub provider_id: String,
    pub context_window: i64,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub input_price: f64,
    pub output_price: f64,
}

fn now_string() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

impl Database {
    /// Inserts or refreshes a discovered model. Favorites and downloaded GGUF
    /// rows are never overwritten by discovery.
    pub fn upsert_discovered_model(&self, model: &DiscoveredModelRecord) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO models
                 (id, name, provider_type, provider_id, context_window, supports_tools,
                  supports_streaming, input_price, output_price, download_status, discovered_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'remote_only', ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    provider_type = excluded.provider_type,
                    provider_id = excluded.provider_id,
                    context_window = excluded.context_window,
                    supports_tools = excluded.supports_tools,
                    supports_streaming = excluded.supports_streaming,
                    input_price = excluded.input_price,
                    output_price = excluded.output_price,
                    discovered_at = excluded.discovered_at",
                params![
                    model.id,
                    model.name,
                    model.provider_type,
                    model.provider_id,
                    model.context_window,
                    i64::from(model.supports_tools),
                    i64::from(model.supports_streaming),
                    model.input_price,
                    model.output_price,
                    now_string(),
                ],
            )?;
            Ok(())
        })
    }

    /// Marks provider models that disappeared from the catalog `remote_only`,
    /// keeping favorites and models referenced by missions intact.
    pub fn mark_missing_discovered_models(
        &self,
        provider_id: &str,
        seen_ids: &[String],
    ) -> Result<usize, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id FROM models WHERE provider_id = ?1 AND is_favorite = 0
                   AND id NOT IN (SELECT DISTINCT active_model_id FROM missions)",
            )?;
            let ids: Vec<String> = stmt
                .query_map([provider_id], |row| row.get(0))?
                .filter_map(Result::ok)
                .collect();
            let mut updated = 0;
            for id in ids {
                if !seen_ids.contains(&id) {
                    updated += conn.execute(
                        "UPDATE models SET download_status = 'remote_only' WHERE id = ?1",
                        params![id],
                    )?;
                }
            }
            Ok(updated)
        })
    }

    pub fn set_model_favorite(&self, model_id: &str, favorite: bool) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE models SET is_favorite = ?1 WHERE id = ?2",
                params![i64::from(favorite), model_id],
            )?;
            Ok(())
        })
    }

    pub fn set_mission_model(&self, mission_id: i64, model_id: &str) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE missions SET active_model_id = ?1 WHERE id = ?2",
                params![model_id, mission_id],
            )?;
            Ok(())
        })
    }
}
