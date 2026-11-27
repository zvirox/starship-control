use crate::Database;
use crate::schema::{MissionMessageRecord, MissionRecord, ModelRecord, ProviderConfigRecord};
use rusqlite::params;
use starship_core::CoreError;

/// A decrypted secret held only at the point of use. `Debug` is redacted so an
/// accidental log or panic cannot leak an API key (Phase 17, task 17.11).
#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl Database {
    /// Retrieves all registered models from the SQLite store.
    pub fn list_models(&self) -> Result<Vec<ModelRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, provider_type, hf_repo_id, file_path, file_size_bytes, 
                        quantization, sha256_hash, download_status, context_window, gpu_layers, 
                        is_default, provider_id, supports_tools, supports_streaming,
                        input_price, output_price, is_favorite, created_at, updated_at
                 FROM models ORDER BY is_default DESC, name ASC, is_favorite DESC",
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
                    provider_id: row.get(12)?,
                    supports_tools: row.get::<_, i64>(13)? == 1,
                    supports_streaming: row.get::<_, i64>(14)? == 1,
                    input_price: row.get(15)?,
                    output_price: row.get(16)?,
                    is_favorite: row.get::<_, i64>(17)? == 1,
                    created_at: row.get(18)?,
                    updated_at: row.get(19)?,
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
                        is_default, provider_id, supports_tools, supports_streaming,
                        input_price, output_price, is_favorite, created_at, updated_at
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
                    provider_id: row.get(12)?,
                    supports_tools: row.get::<_, i64>(13)? == 1,
                    supports_streaming: row.get::<_, i64>(14)? == 1,
                    input_price: row.get(15)?,
                    output_price: row.get(16)?,
                    is_favorite: row.get::<_, i64>(17)? == 1,
                    created_at: row.get(18)?,
                    updated_at: row.get(19)?,
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
                        total_input_tokens, total_output_tokens, cost_usd, created_at, updated_at, archived_at,
                        peak_cpu_pct, peak_ram_bytes, peak_temp_celsius, governor_interventions
                 FROM missions WHERE id = ?1",
            )?;

            let mut rows = stmt.query_map(params![id], |row| {
                let peak_cpu: f64 = row.get(13)?;
                let peak_ram: i64 = row.get(14)?;
                let peak_temp: f64 = row.get(15)?;
                #[allow(clippy::cast_possible_truncation)]
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
                    peak_cpu_pct: peak_cpu as f32,
                    peak_ram_bytes: peak_ram.max(0).cast_unsigned(),
                    peak_temp_celsius: peak_temp as f32,
                    governor_interventions: row.get(16)?,
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
                        total_input_tokens, total_output_tokens, cost_usd, created_at, updated_at, archived_at,
                        peak_cpu_pct, peak_ram_bytes, peak_temp_celsius, governor_interventions
                 FROM missions WHERE archived_at IS NULL ORDER BY updated_at DESC, id DESC",
            )?;

            let rows = stmt.query_map([], |row| {
                let peak_cpu: f64 = row.get(13)?;
                let peak_ram: i64 = row.get(14)?;
                let peak_temp: f64 = row.get(15)?;
                #[allow(clippy::cast_possible_truncation)]
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
                    peak_cpu_pct: peak_cpu as f32,
                    peak_ram_bytes: peak_ram.max(0).cast_unsigned(),
                    peak_temp_celsius: peak_temp as f32,
                    governor_interventions: row.get(16)?,
                })
            })?;

            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            Ok(list)
        })
    }

    /// Atomically records health metrics and increments governor interventions for a mission (GAP-CLOSURE-PLAN.md §14.11).
    pub fn record_mission_health_sample(
        &self,
        mission_id: i64,
        cpu_pct: f32,
        ram_bytes: u64,
        temp_celsius: f32,
        is_governor_intervention: bool,
    ) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            let interventions = i64::from(is_governor_intervention);
            #[allow(clippy::cast_possible_wrap)]
            conn.execute(
                "UPDATE missions SET
                    peak_cpu_pct = MAX(peak_cpu_pct, ?1),
                    peak_ram_bytes = MAX(peak_ram_bytes, ?2),
                    peak_temp_celsius = MAX(peak_temp_celsius, ?3),
                    governor_interventions = governor_interventions + ?4,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?5",
                params![
                    f64::from(cpu_pct),
                    ram_bytes as i64,
                    f64::from(temp_celsius),
                    interventions,
                    mission_id
                ],
            )?;
            Ok(())
        })
    }

    /// Updates all health summary metrics for a mission directly (GAP-CLOSURE-PLAN.md §14.11).
    pub fn set_mission_health_summary(
        &self,
        mission_id: i64,
        peak_cpu_pct: f32,
        peak_ram_bytes: u64,
        peak_temp_celsius: f32,
        governor_interventions: i64,
    ) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            #[allow(clippy::cast_possible_wrap)]
            conn.execute(
                "UPDATE missions SET
                    peak_cpu_pct = ?1,
                    peak_ram_bytes = ?2,
                    peak_temp_celsius = ?3,
                    governor_interventions = ?4,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?5",
                params![
                    f64::from(peak_cpu_pct),
                    peak_ram_bytes as i64,
                    f64::from(peak_temp_celsius),
                    governor_interventions,
                    mission_id
                ],
            )?;
            Ok(())
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

    /// Lists all configured cloud providers (metadata only; keys stay encrypted).
    pub fn list_provider_configs(&self) -> Result<Vec<ProviderConfigRecord>, CoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT provider_id, display_name, base_url, api_key_encrypted, is_enabled, created_at, updated_at
                 FROM provider_configs ORDER BY provider_id",
            )?;
            let rows = stmt.query_map([], |row| {
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
            rows.collect::<Result<Vec<_>, _>>()
        })
    }

    /// Sets or updates a cloud provider configuration.
    /// Automatically encrypts plaintext API keys with `SecretVault` (GAP-CLOSURE-PLAN.md §10.7).
    pub fn set_provider_config(
        &self,
        provider_id: &str,
        display_name: &str,
        base_url: &str,
        api_key_encrypted: Option<&str>,
        is_enabled: bool,
    ) -> Result<(), CoreError> {
        let armored_key = api_key_encrypted.map(|key| {
            if key.starts_with("enc_v1:") {
                key.to_string()
            } else {
                starship_core::SecretVault::encrypt_secret(key, "starship-db-provider-key-salt")
            }
        });

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
                params![provider_id, display_name, base_url, armored_key.as_deref(), i32::from(is_enabled)],
            )?;
            Ok(())
        })
    }

    /// Loads and decrypts a provider's API key for immediate use.
    /// The returned value must never be logged, persisted or put in a prompt.
    pub fn get_provider_credentials(
        &self,
        provider_id: &str,
    ) -> Result<Option<SecretString>, CoreError> {
        let cfg = self.get_provider_config(provider_id)?;
        match cfg.and_then(|c| c.api_key_encrypted) {
            Some(armored) => Ok(Some(SecretString::new(Self::decrypt_api_key(&armored)?))),
            None => Ok(None),
        }
    }

    /// Marks local GGUF rows whose file is missing as not downloaded.
    pub fn mark_missing_local_models(&self) -> Result<usize, CoreError> {
        let models = self.list_models()?;
        let mut updated = 0;
        for model in models {
            if model.provider_type == "local_gguf"
                && model.download_status == "ready"
                && !model
                    .file_path
                    .as_ref()
                    .is_some_and(|path| std::path::Path::new(path).exists())
            {
                updated += self.with_conn(|conn| {
                    conn.execute(
                        "UPDATE models SET download_status = 'remote_only' WHERE id = ?1",
                        params![model.id],
                    )
                })?;
            }
        }
        Ok(updated)
    }

    /// Deletes a model row. Referenced models are rejected by the FK constraint.
    pub fn delete_model(&self, model_id: &str) -> Result<(), CoreError> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM models WHERE id = ?1", params![model_id])?;
            Ok(())
        })
    }

    /// Stores a tool result as a `tool` message for the next model turn.
    pub fn add_tool_message(
        &self,
        mission_id: i64,
        tool_call_id: &str,
        content: &str,
    ) -> Result<i64, CoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO mission_messages (mission_id, role, content, tool_call_id)
                 VALUES (?1, 'tool', ?2, ?3)",
                params![mission_id, content, tool_call_id],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    /// Decrypts an armored API key from the database.
    pub fn decrypt_api_key(armored: &str) -> Result<String, CoreError> {
        starship_core::SecretVault::decrypt_secret(armored, "starship-db-provider-key-salt")
    }
}
