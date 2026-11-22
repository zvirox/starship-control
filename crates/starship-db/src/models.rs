use crate::Database;
use crate::schema::{MissionRecord, ModelRecord};
use rusqlite::params;
use starship_core::CoreError;

impl Database {
    /// Retrieves all registered models from the SQLite store.
    pub fn list_models(&self) -> Result<Vec<ModelRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, provider_type, hf_repo_id, file_path, file_size_bytes, 
                        quantization, sha256_hash, download_status, context_window, gpu_layers, 
                        is_default, created_at, updated_at 
                 FROM models ORDER BY is_default DESC, name ASC",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(ModelRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    provider_type: row.get(2)?,
                    hf_repo_id: row.get(3)?,
                    file_path: row.get(4)?,
                    file_size_bytes: row.get(5)?,
                    quantization: row.get(6)?,
                    sha256_hash: row.get(7)?,
                    download_status: row.get(8)?,
                    context_window: row.get(9)?,
                    gpu_layers: row.get(10)?,
                    is_default: row.get::<_, i64>(11)? == 1,
                    created_at: row.get(12)?,
                    updated_at: row.get(13)?,
                })
            })?;

            let mut models = Vec::new();
            for r in rows {
                models.push(r?);
            }
            Ok(models)
        })
    }

    /// Creates a new mission and returns its inserted row ID.
    pub fn create_mission(
        &self,
        uuid: &str,
        title: &str,
        user_goal: &str,
        active_model_id: &str,
        provider_type: &str,
    ) -> Result<i64, CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO missions (uuid, title, user_goal, active_model_id, provider_type)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![uuid, title, user_goal, active_model_id, provider_type],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    /// Fetches a mission by ID.
    pub fn get_mission(&self, id: i64) -> Result<Option<MissionRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, uuid, title, user_goal, active_model_id, provider_type, status,
                        total_input_tokens, total_output_tokens, cost_usd, created_at, updated_at, archived_at
                 FROM missions WHERE id = ?1",
            )?;

            let mut rows = stmt.query_map(params![id], |row| {
                Ok(MissionRecord {
                    id: row.get(0)?,
                    uuid: row.get(1)?,
                    title: row.get(2)?,
                    user_goal: row.get(3)?,
                    active_model_id: row.get(4)?,
                    provider_type: row.get(5)?,
                    status: row.get(6)?,
                    total_input_tokens: row.get(7)?,
                    total_output_tokens: row.get(8)?,
                    cost_usd: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                    archived_at: row.get(12)?,
                })
            })?;

            if let Some(res) = rows.next() {
                Ok(Some(res?))
            } else {
                Ok(None)
            }
        })
    }
}
