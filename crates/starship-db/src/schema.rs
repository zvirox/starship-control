use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MissionRecord {
    pub id: i64,
    pub uuid: String,
    pub title: String,
    pub user_goal: String,
    pub active_model_id: String,
    pub provider_type: String,
    pub status: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub cost_usd: f64,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    pub peak_cpu_pct: f32,
    pub peak_ram_bytes: u64,
    pub peak_temp_celsius: f32,
    pub governor_interventions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MissionMessageRecord {
    pub id: i64,
    pub mission_id: i64,
    pub role: String,
    pub content: String,
    pub thought_process: Option<String>,
    pub tool_calls_json: Option<String>,
    pub tool_call_id: Option<String>,
    pub tokens_used: Option<i64>,
    pub latency_ms: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanRecord {
    pub id: i64,
    pub mission_id: i64,
    pub version: i64,
    pub title: String,
    pub summary: String,
    pub raw_tasksheet_json: String,
    pub status: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlanStepRecord {
    pub id: i64,
    pub plan_id: i64,
    pub step_index: i64,
    pub app_id: String,
    pub tool_name: String,
    pub arguments_json: String,
    pub risk_tier: i64,
    pub approval_status: String,
    pub execution_status: String,
    pub diff_summary: Option<String>,
    pub result_json: Option<String>,
    pub error_message: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppRegistryRecord {
    pub app_id: String,
    pub display_name: String,
    pub version: String,
    pub binary_path: String,
    pub manifest_path: String,
    pub socket_path: Option<String>,
    pub capabilities_json: String,
    pub is_active_gui: bool,
    pub last_seen_at: String,
    pub created_at: String,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRecord {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub hf_repo_id: Option<String>,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub quantization: Option<String>,
    pub sha256_hash: Option<String>,
    pub download_status: String,
    pub context_window: i64,
    pub gpu_layers: i64,
    pub is_default: bool,
    pub provider_id: Option<String>,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub input_price: f64,
    pub output_price: f64,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfigRecord {
    pub provider_id: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditLogRecord {
    pub id: i64,
    pub mission_id: Option<i64>,
    pub app_id: String,
    pub action_type: String,
    pub risk_tier: i64,
    pub payload_json: String,
    pub status: String,
    pub signature_token: Option<String>,
    pub created_at: String,
}
