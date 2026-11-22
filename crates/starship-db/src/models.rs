use crate::Database;
use crate::schema::{MissionMessageRecord, MissionRecord, ModelRecord, ProviderConfigRecord};
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

    /// Registers or updates a downloaded model file in the models table.
    #[allow(clippy::too_many_arguments)]
    pub fn register_model(
        &self,
        id: &str,
        name: &str,
        provider_type: &str,
        hf_repo_id: Option<&str>,
        file_path: Option<&str>,
        file_size_bytes: Option<i64>,
        quantization: Option<&str>,
        sha256_hash: Option<&str>,
        context_window: i64,
        gpu_layers: i64,
    ) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO models (
                    id, name, provider_type, hf_repo_id, file_path, file_size_bytes,
                    quantization, sha256_hash, download_status, context_window, gpu_layers,
                    is_default, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'ready', ?9, ?10, 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    file_path = excluded.file_path,
                    file_size_bytes = excluded.file_size_bytes,
                    sha256_hash = excluded.sha256_hash,
                    download_status = 'ready',
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
                params![
                    id,
                    name,
                    provider_type,
                    hf_repo_id,
                    file_path,
                    file_size_bytes,
                    quantization,
                    sha256_hash,
                    context_window,
                    gpu_layers,
                ],
            )?;
            Ok(())
        })
    }

    /// Fetches a specific model by ID.
    pub fn get_model(&self, id: &str) -> Result<Option<ModelRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, provider_type, hf_repo_id, file_path, file_size_bytes, 
                        quantization, sha256_hash, download_status, context_window, gpu_layers, 
                        is_default, created_at, updated_at 
                 FROM models WHERE id = ?1",
            )?;

            let mut rows = stmt.query_map(params![id], |row| {
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

            if let Some(r) = rows.next() {
                Ok(Some(r?))
            } else {
                Ok(None)
            }
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

    /// Lists all active missions ordered by latest update.
    pub fn list_missions(&self) -> Result<Vec<MissionRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, uuid, title, user_goal, active_model_id, provider_type, status,
                        total_input_tokens, total_output_tokens, cost_usd, created_at, updated_at, archived_at
                 FROM missions WHERE archived_at IS NULL ORDER BY updated_at DESC, id DESC",
            )?;

            let rows = stmt.query_map([], |row| {
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

            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            Ok(list)
        })
    }

    /// Deletes a mission and its associated messages.
    pub fn delete_mission(&self, id: i64) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM mission_messages WHERE mission_id = ?1", params![id])?;
            conn.execute("DELETE FROM plan_steps WHERE plan_id IN (SELECT id FROM plans WHERE mission_id = ?1)", params![id])?;
            conn.execute("DELETE FROM plans WHERE mission_id = ?1", params![id])?;
            conn.execute("DELETE FROM missions WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    /// Appends a message to the mission messages log.
    pub fn add_message(
        &self,
        mission_id: i64,
        role: &str,
        content: &str,
        thought: Option<&str>,
    ) -> Result<i64, CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO mission_messages (mission_id, role, content, thought_process)
                 VALUES (?1, ?2, ?3, ?4)",
                params![mission_id, role, content, thought],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    /// Lists messages for a given mission chronologically.
    pub fn list_messages(&self, mission_id: i64) -> Result<Vec<MissionMessageRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, mission_id, role, content, thought_process, tool_calls_json, tool_call_id,
                        tokens_used, latency_ms, created_at
                 FROM mission_messages WHERE mission_id = ?1 ORDER BY created_at ASC, id ASC",
            )?;

            let rows = stmt.query_map(params![mission_id], |row| {
                Ok(MissionMessageRecord {
                    id: row.get(0)?,
                    mission_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    thought_process: row.get(4)?,
                    tool_calls_json: row.get(5)?,
                    tool_call_id: row.get(6)?,
                    tokens_used: row.get(7)?,
                    latency_ms: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })?;

            let mut msgs = Vec::new();
            for r in rows {
                msgs.push(r?);
            }
            Ok(msgs)
        })
    }

    /// Retrieves provider configuration for a cloud provider.
    pub fn get_provider_config(
        &self,
        provider_id: &str,
    ) -> Result<Option<ProviderConfigRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT provider_id, display_name, base_url, api_key_encrypted, is_enabled, created_at, updated_at
                 FROM provider_configs WHERE provider_id = ?1",
            )?;

            let mut rows = stmt.query_map(params![provider_id], |row| {
                Ok(ProviderConfigRecord {
                    provider_id: row.get(0)?,
                    display_name: row.get(1)?,
                    base_url: row.get(2)?,
                    api_key_encrypted: row.get(3)?,
                    is_enabled: row.get::<_, i64>(4)? == 1,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?;

            if let Some(res) = rows.next() {
                Ok(Some(res?))
            } else {
                Ok(None)
            }
        })
    }

    /// Sets or updates a cloud provider configuration.
    pub fn set_provider_config(
        &self,
        provider_id: &str,
        display_name: &str,
        base_url: &str,
        api_key_encrypted: Option<&str>,
        is_enabled: bool,
    ) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO provider_configs (provider_id, display_name, base_url, api_key_encrypted, is_enabled)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(provider_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    base_url = excluded.base_url,
                    api_key_encrypted = COALESCE(excluded.api_key_encrypted, provider_configs.api_key_encrypted),
                    is_enabled = excluded.is_enabled,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
                params![provider_id, display_name, base_url, api_key_encrypted, i32::from(is_enabled)],
            )?;
            Ok(())
        })
    }
}
