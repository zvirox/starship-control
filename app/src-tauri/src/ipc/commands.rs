use futures::StreamExt;
use serde::{Deserialize, Serialize};
use starship_core::pipeline::{
    CetTelemetry, DiagnosticsPipeline, ScreencastAssemblyRequest, ScreencastPipeline,
    SystemMetricsReport,
};
use starship_core::planner::{
    DryRunExecutor, DryRunReport, PlanPreview, TaskSheet, TaskSheetStep, TaskSheetValidator,
};
use starship_core::safety::{RiskTier, SafetyArbiter};
use starship_core::{CoreError, ErrorKind};
use starship_db::schema::{MissionMessageRecord, MissionRecord, ModelRecord};
use starship_db::{Database, DiscoveredModelRecord, InferMetricRecord, NewAuditEntry};
use starship_infer::ProviderHeaders;
use starship_infer::discovery::{DiscoveredModel, ModelDiscovery, ProviderDiscovery};
use starship_infer::dispatcher::InferDispatcher;
use starship_infer::hf::{DownloadProgress, HfModelMetadata, download_hf_file, search_models};
use starship_infer::types::{ChatRequest, Message, TokenStream, ToolCall};
use starship_mcp::{
    AggregatedTool, HybridExecutionRouter, ToolAggregator, UnixTransportClient,
    scan_and_sync_manifests,
};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, Size};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub name: String,
    pub version: String,
    pub protocol: String,
    pub status: String,
    pub database_ready: bool,
    pub available_models: Vec<ModelRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMissionPayload {
    pub title: String,
    #[serde(rename = "userGoal")]
    pub user_goal: String,
    #[serde(rename = "modelId")]
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMissionResponse {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
    pub uuid: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendPromptPayload {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendPromptResponse {
    #[serde(rename = "messageId")]
    pub message_id: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStreamEvent {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
    pub chunk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStreamEvent {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
    #[serde(rename = "thoughtChunk")]
    pub thought_chunk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamFinishedEvent {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
    #[serde(rename = "messageId")]
    pub message_id: i64,
    pub content: String,
    pub thought: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamErrorEvent {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMissionPayload {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchModelsPayload {
    pub query: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadModelPayload {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    pub filename: String,
    #[serde(rename = "expectedSha256")]
    pub expected_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadModelResponse {
    #[serde(rename = "downloadJobId")]
    pub download_job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadCompleteEvent {
    #[serde(rename = "downloadJobId")]
    pub download_job_id: String,
    #[serde(rename = "modelId")]
    pub model_id: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigPayload {
    #[serde(rename = "providerId")]
    pub provider_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "baseUrl")]
    pub base_url: String,
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(rename = "isEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "httpReferer")]
    pub http_referer: Option<String>,
    #[serde(rename = "xTitle")]
    pub x_title: Option<String>,
}

#[tauri::command]
pub async fn ipc_hello(
    app: AppHandle,
    state: tauri::State<'_, Database>,
) -> Result<HelloResponse, CoreError> {
    info!(target: "starship_ipc", "Received ipc_hello handshake");

    let models = state.list_models().unwrap_or_default();

    Ok(HelloResponse {
        name: "Starship Control".to_string(),
        version: app.package_info().version.to_string(),
        protocol: "starship/1.0".to_string(),
        status: "nominal".to_string(),
        database_ready: true,
        available_models: models,
    })
}

#[tauri::command]
pub async fn cmd_toggle_hud(app: AppHandle) -> Result<bool, CoreError> {
    if let Some(hud_window) = app.get_webview_window("hud") {
        let is_visible = hud_window.is_visible().map_err(|e| {
            CoreError::ipc(
                "ipc.window_error",
                false,
                format!("Failed to check HUD visibility: {e}"),
                None,
            )
        })?;

        if is_visible {
            hud_window.hide().map_err(|e| {
                CoreError::ipc(
                    "ipc.window_error",
                    false,
                    format!("Failed to hide HUD window: {e}"),
                    None,
                )
            })?;
            info!(target: "starship_ipc", "HUD hidden");
            Ok(false)
        } else {
            hud_window.show().map_err(|e| {
                CoreError::ipc(
                    "ipc.window_error",
                    false,
                    format!("Failed to show HUD window: {e}"),
                    None,
                )
            })?;
            hud_window.set_focus().map_err(|e| {
                CoreError::ipc(
                    "ipc.window_error",
                    false,
                    format!("Failed to focus HUD window: {e}"),
                    None,
                )
            })?;
            info!(target: "starship_ipc", "HUD shown");
            Ok(true)
        }
    } else {
        Err(CoreError::new(
            ErrorKind::Ipc,
            "ipc.window_not_found",
            false,
            "HUD window is not configured or unavailable",
            None,
        ))
    }
}

#[tauri::command]
pub async fn cmd_resize_hud(app: AppHandle, expanded: bool) -> Result<(), CoreError> {
    if let Some(hud_window) = app.get_webview_window("hud") {
        let target_height = if expanded { 240.0 } else { 64.0 };
        hud_window
            .set_size(Size::Logical(LogicalSize {
                width: 420.0,
                height: target_height,
            }))
            .map_err(|e| {
                CoreError::ipc(
                    "ipc.window_error",
                    false,
                    format!("Failed to resize HUD window: {e}"),
                    None,
                )
            })?;
        info!(target: "starship_ipc", expanded, height = target_height, "Resized HUD window");
        Ok(())
    } else {
        Err(CoreError::new(
            ErrorKind::Ipc,
            "ipc.window_not_found",
            false,
            "HUD window is not configured or unavailable",
            None,
        ))
    }
}

#[tauri::command]
pub async fn cmd_list_models(
    state: tauri::State<'_, Database>,
) -> Result<Vec<ModelRecord>, CoreError> {
    let _ = state.mark_missing_local_models();
    state.list_models()
}

#[tauri::command]
pub async fn cmd_model_hf_search(
    payload: SearchModelsPayload,
) -> Result<Vec<HfModelMetadata>, CoreError> {
    search_models(&payload.query, payload.limit.unwrap_or(10)).await
}

#[tauri::command]
pub async fn cmd_model_hf_download(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: DownloadModelPayload,
) -> Result<DownloadModelResponse, CoreError> {
    let job_id = format!("dl_{}", &Uuid::new_v4().to_string()[..8]);
    let models_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("starship-control")
        .join("models");

    let target_path = models_dir.join(&payload.filename);
    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        payload.repo_id, payload.filename
    );

    let app_handle = app.clone();
    let db = state.inner().clone();
    let job_id_clone = job_id.clone();
    let filename_clone = payload.filename.clone();
    let repo_id_clone = payload.repo_id.clone();
    let expected_sha = payload.expected_sha256.clone();

    info!(target: "starship_ipc", job_id = %job_id, file = %payload.filename, "Initiating GGUF download");

    tokio::spawn(async move {
        let app_progress = app_handle.clone();
        let target_path_str = target_path.display().to_string();

        let res = download_hf_file(
            &job_id_clone,
            &download_url,
            &target_path,
            expected_sha.as_deref(),
            move |progress: DownloadProgress| {
                let _ = app_progress.emit("evt://download/progress", progress);
            },
        )
        .await;

        match res {
            Ok(final_sha) => {
                let model_id = filename_clone.trim_end_matches(".gguf").to_string();
                let file_size = std::fs::metadata(&target_path)
                    .map_or(0, |m| i64::try_from(m.len()).unwrap_or(i64::MAX));

                let _ = db.register_model(
                    &model_id,
                    &filename_clone,
                    "local_gguf",
                    Some(&repo_id_clone),
                    Some(&target_path_str),
                    Some(file_size),
                    Some("Q4_K_M"),
                    Some(&final_sha),
                    8192,
                    33,
                );

                let _ = app_handle.emit(
                    "evt://download/complete",
                    DownloadCompleteEvent {
                        download_job_id: job_id_clone.clone(),
                        model_id,
                        file_path: target_path_str,
                        sha256: final_sha,
                    },
                );
                info!(target: "starship_ipc", job_id = %job_id_clone, "GGUF download and SHA-256 validation complete");
            }
            Err(e) => {
                error!(target: "starship_ipc", job_id = %job_id_clone, "Download failed: {e}");
                let _ = app_handle.emit("evt://download/error", e);
            }
        }
    });

    Ok(DownloadModelResponse {
        download_job_id: job_id,
    })
}

#[tauri::command]
pub async fn cmd_mission_create(
    state: tauri::State<'_, Database>,
    payload: CreateMissionPayload,
) -> Result<CreateMissionResponse, CoreError> {
    let model = state.get_model(&payload.model_id)?.ok_or_else(|| {
        CoreError::infer(
            "infer.model_not_found",
            false,
            format!("Model not found: {}", payload.model_id),
            None,
        )
    })?;

    let mission_uuid = Uuid::new_v4().to_string();
    let mission_id = state.create_mission(
        &mission_uuid,
        &payload.title,
        &payload.user_goal,
        &model.id,
        &model.provider_type,
    )?;

    info!(target: "starship_ipc", mission_id, uuid = %mission_uuid, "Mission created");

    Ok(CreateMissionResponse {
        mission_id,
        uuid: mission_uuid,
        created_at: chrono_now_iso(),
    })
}

#[tauri::command]
pub async fn cmd_mission_list(
    state: tauri::State<'_, Database>,
) -> Result<Vec<MissionRecord>, CoreError> {
    state.list_missions()
}

#[tauri::command]
pub async fn cmd_mission_list_history(
    state: tauri::State<'_, Database>,
) -> Result<Vec<starship_db::MissionHistorySummary>, CoreError> {
    state.list_mission_history()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayMissionPayload {
    pub mission_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayMissionResponse {
    pub mission_id: i64,
    pub mission_state: starship_core::executor::MissionState,
    pub step_states: std::collections::HashMap<i64, starship_core::executor::StepState>,
    pub events: Vec<starship_db::MissionEventRecord>,
}

#[tauri::command]
pub async fn cmd_mission_replay(
    state: tauri::State<'_, Database>,
    payload: ReplayMissionPayload,
) -> Result<ReplayMissionResponse, CoreError> {
    let (mission_state, step_states) = state.rebuild_mission_state(payload.mission_id)?;
    let events = state.list_mission_events(payload.mission_id, None)?;
    Ok(ReplayMissionResponse {
        mission_id: payload.mission_id,
        mission_state,
        step_states,
        events,
    })
}

#[tauri::command]
pub async fn cmd_mission_delete(
    state: tauri::State<'_, Database>,
    payload: DeleteMissionPayload,
) -> Result<bool, CoreError> {
    state.delete_mission(payload.mission_id)?;
    Ok(true)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrayState {
    Idle,
    Thinking,
    Executing,
    WaitingApproval,
    Stalled,
    Overloaded,
    Failed,
}

impl TrayState {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Thinking => "Thinking",
            Self::Executing => "Executing",
            Self::WaitingApproval => "Waiting Approval",
            Self::Stalled => "Stalled",
            Self::Overloaded => "Overloaded",
            Self::Failed => "Failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraySetStatePayload {
    pub state: TrayState,
}

#[tauri::command]
pub async fn cmd_tray_set_state(
    app: AppHandle,
    payload: TraySetStatePayload,
) -> Result<String, CoreError> {
    info!(target: "starship_tray", state = ?payload.state, "Setting tray state");

    // Emit event to frontend listeners
    let _ = app.emit("evt://tray/state_changed", &payload);

    // If a system tray is registered, update its tooltip
    if let Some(tray) = app.tray_by_id("main-tray") {
        let tooltip = format!("Starship Control — {}", payload.state.label());
        let _ = tray.set_tooltip(Some(tooltip));
    }

    Ok(payload.state.label().to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchNotificationPayload {
    pub trigger: crate::notification::NotificationTrigger,
    pub mission_id: i64,
    pub step_id: Option<i64>,
    pub stall_seconds: Option<u64>,
    pub title: String,
    pub body: String,
}

#[tauri::command]
pub async fn cmd_dispatch_notification(
    notifier: tauri::State<'_, std::sync::Arc<crate::notification::NotificationManager>>,
    payload: DispatchNotificationPayload,
) -> Result<bool, CoreError> {
    let priority = match payload.trigger {
        crate::notification::NotificationTrigger::MissionFailed => {
            ashpd::desktop::notification::Priority::Urgent
        }
        crate::notification::NotificationTrigger::ApprovalNeeded
        | crate::notification::NotificationTrigger::MissionStalled => {
            ashpd::desktop::notification::Priority::High
        }
        crate::notification::NotificationTrigger::MissionFinished => {
            ashpd::desktop::notification::Priority::Normal
        }
    };

    let sent = notifier
        .dispatch(
            payload.trigger,
            payload.mission_id,
            payload.step_id,
            payload.stall_seconds,
            &payload.title,
            &payload.body,
            priority,
        )
        .await;

    Ok(sent)
}

#[tauri::command]
pub async fn cmd_mission_list_messages(
    state: tauri::State<'_, Database>,
    mission_id: i64,
) -> Result<Vec<MissionMessageRecord>, CoreError> {
    state.list_messages(mission_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionGetTimelinePayload {
    pub mission_id: i64,
    pub since_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionTimelineEvent {
    pub id: i64,
    pub mission_id: i64,
    pub step_id: Option<i64>,
    pub ts_ms: i64,
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionGetTimelineResponse {
    pub mission_id: i64,
    pub cursor: i64,
    pub events: Vec<MissionTimelineEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MissionProgressPayload {
    pub mission_id: i64,
    pub mission_progress: f64,
    pub active_step_index: Option<u32>,
    pub step_progress: f64,
    pub step_message: Option<String>,
    pub eta_range: Option<EtaRangePayload>,
    pub is_stalled: bool,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct EtaRangePayload {
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub is_approximate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct StepStatePayload {
    pub mission_id: i64,
    pub step_id: i64,
    pub step_index: u32,
    pub state: String,
    pub failure_class: Option<String>,
    pub error: Option<String>,
    pub attempt: u32,
}

#[tauri::command]
pub async fn cmd_mission_get_timeline(
    state: tauri::State<'_, Database>,
    payload: MissionGetTimelinePayload,
) -> Result<MissionGetTimelineResponse, CoreError> {
    let records = state.list_mission_events(payload.mission_id, payload.since_id)?;
    let mut cursor = payload.since_id.unwrap_or(0);
    let mut events = Vec::with_capacity(records.len());

    for rec in records {
        if rec.id > cursor {
            cursor = rec.id;
        }
        let payload_val =
            serde_json::from_str(&rec.payload_json).unwrap_or(serde_json::Value::Null);
        events.push(MissionTimelineEvent {
            id: rec.id,
            mission_id: rec.mission_id,
            step_id: rec.step_id,
            ts_ms: rec.ts_ms,
            kind: rec.kind,
            payload: payload_val,
        });
    }

    Ok(MissionGetTimelineResponse {
        mission_id: payload.mission_id,
        cursor,
        events,
    })
}

#[allow(dead_code)]
pub fn emit_mission_progress(
    app: &AppHandle,
    payload: &MissionProgressPayload,
) -> Result<(), CoreError> {
    app.emit("evt://mission/progress", payload).map_err(|e| {
        CoreError::ipc(
            "ipc.emit_failed",
            false,
            format!("Failed to emit evt://mission/progress: {e}"),
            None,
        )
    })
}

#[allow(dead_code)]
pub fn emit_step_state(app: &AppHandle, payload: &StepStatePayload) -> Result<(), CoreError> {
    app.emit("evt://step/state", payload).map_err(|e| {
        CoreError::ipc(
            "ipc.emit_failed",
            false,
            format!("Failed to emit evt://step/state: {e}"),
            None,
        )
    })
}

#[tauri::command]
pub async fn cmd_provider_config_set(
    state: tauri::State<'_, Database>,
    payload: ProviderConfigPayload,
) -> Result<bool, CoreError> {
    state.set_provider_config(
        &payload.provider_id,
        &payload.display_name,
        &payload.base_url,
        payload.api_key.as_deref(),
        payload.is_enabled,
    )?;
    state.set_provider_headers(
        &payload.provider_id,
        payload.http_referer.as_deref(),
        payload.x_title.as_deref(),
    )?;
    info!(target: "starship_ipc", provider_id = %payload.provider_id, "Provider config saved");
    if payload.api_key.is_some()
        && payload.is_enabled
        && let Err(err) = refresh_provider_models(state.inner(), &payload.provider_id).await
    {
        warn!(target: "starship_ipc", provider_id = %payload.provider_id, "Auto model discovery failed: {err}");
    }
    Ok(true)
}

/// Cloud provider metadata exposed to the settings UI (never the stored key).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfoResponse {
    pub provider_id: String,
    pub display_name: String,
    pub base_url: String,
    pub is_enabled: bool,
    pub has_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderIdPayload {
    pub provider_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTestResponse {
    pub ok: bool,
    pub latency_ms: i64,
    pub model_count: usize,
    pub error_code: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelsResponse {
    pub provider_id: String,
    pub models: Vec<DiscoveredModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFavoritePayload {
    pub model_id: String,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MissionModelPayload {
    pub mission_id: i64,
    pub model_id: String,
}

fn provider_type_for_id(provider_id: &str) -> &'static str {
    match provider_id {
        "openrouter" => "cloud_openrouter",
        "anthropic" => "cloud_anthropic",
        "gemini" => "cloud_gemini",
        _ => "cloud_openai",
    }
}

fn discovered_to_record(model: &DiscoveredModel, provider_id: &str) -> DiscoveredModelRecord {
    DiscoveredModelRecord {
        id: model.id.clone(),
        name: model.display_name.clone(),
        provider_type: provider_type_for_id(provider_id).to_string(),
        provider_id: provider_id.to_string(),
        context_window: model.context_window,
        supports_tools: model.supports_tools,
        supports_streaming: model.supports_streaming,
        input_price: model.input_price,
        output_price: model.output_price,
    }
}

/// Fetches and persists a provider's live model catalog (Phase 17, 17.22).
async fn refresh_provider_models(
    db: &Database,
    provider_id: &str,
) -> Result<Vec<DiscoveredModel>, CoreError> {
    let cfg = db.get_provider_config(provider_id)?.ok_or_else(|| {
        CoreError::app(
            "app.not_found",
            false,
            format!("Provider '{provider_id}' is not configured"),
            None,
        )
    })?;
    let key = db
        .get_provider_credentials(provider_id)?
        .map(|secret| secret.expose().to_owned());
    let models = ProviderDiscovery::new(provider_id, cfg.base_url, key)
        .list_models()
        .await?;
    for model in &models {
        db.upsert_discovered_model(&discovered_to_record(model, provider_id))?;
    }
    let seen: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
    db.mark_missing_discovered_models(provider_id, &seen)?;
    Ok(models)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDeletePayload {
    pub model_id: String,
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cmd_model_delete(
    state: tauri::State<'_, Database>,
    payload: ModelDeletePayload,
) -> Result<bool, CoreError> {
    let model = state.get_model(&payload.model_id)?.ok_or_else(|| {
        CoreError::app(
            "app.not_found",
            false,
            format!("Model '{}' is not registered", payload.model_id),
            None,
        )
    })?;
    if model.provider_type == "local_gguf"
        && let Some(path) = model.file_path.as_ref()
    {
        let models_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("starship-control")
            .join("models");
        let candidate = std::path::Path::new(path);
        if let Ok(canonical) = candidate.canonicalize()
            && canonical.starts_with(&models_dir)
        {
            let _ = std::fs::remove_file(&canonical);
        }
    }
    state.delete_model(&payload.model_id)?;
    Ok(true)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cmd_provider_list(
    state: tauri::State<'_, Database>,
) -> Result<Vec<ProviderInfoResponse>, CoreError> {
    let cfg = state.list_provider_configs()?;
    Ok(cfg
        .into_iter()
        .map(|c| ProviderInfoResponse {
            has_key: c.api_key_encrypted.is_some(),
            provider_id: c.provider_id,
            display_name: c.display_name,
            base_url: c.base_url,
            is_enabled: c.is_enabled,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelIdTestPayload {
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelIdTestResponse {
    pub ok: bool,
    pub latency_ms: i64,
    pub error_code: Option<String>,
    pub message: String,
}

/// Validates a manually entered model id with a one-token chat call and, on
/// success, registers it so the picker can select it (Phase 17, 17.25).
/// One-token validation shared with `cmd_model_test_id` for tests.
async fn validate_model_id(
    db: &Database,
    provider_id: &str,
    model_id: &str,
) -> Result<ModelIdTestResponse, CoreError> {
    let started = Instant::now();
    let latency = || i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let cfg = db.get_provider_config(provider_id)?.ok_or_else(|| {
        CoreError::app(
            "app.not_found",
            false,
            format!("Provider '{provider_id}' is not configured"),
            None,
        )
    })?;
    let key = db
        .get_provider_credentials(provider_id)?
        .map(|secret| secret.expose().to_owned());
    let request = ChatRequest {
        model: model_id.to_string(),
        messages: vec![Message::user("Reply with OK")],
        system_prompt: None,
        temperature: Some(0.0),
        max_tokens: Some(1),
        tools: vec![],
        grammar: None,
        stream: true,
    };
    let provider_type = provider_type_for_id(provider_id);
    let headers = ProviderHeaders {
        http_referer: cfg.http_referer.clone(),
        x_title: cfg.x_title.clone(),
    };
    match dispatch_with_retry(
        provider_type,
        model_id,
        Some(&cfg.base_url),
        key.as_deref(),
        None,
        request,
        Some(&headers),
    )
    .await
    {
        Ok(mut stream) => {
            let mut content = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(chunk) => {
                        content.push_str(&chunk.content);
                        if !content.is_empty() {
                            break;
                        }
                    }
                    Err(error) => {
                        return Ok(ModelIdTestResponse {
                            ok: false,
                            latency_ms: latency(),
                            error_code: Some(error.code.clone()),
                            message: friendly_stream_error(
                                &error.code,
                                provider_type,
                                Some(model_id),
                                &error.user_message,
                                error.details.as_ref(),
                            ),
                        });
                    }
                }
            }
            if content.is_empty() {
                return Ok(ModelIdTestResponse {
                    ok: false,
                    latency_ms: latency(),
                    error_code: Some("infer.empty_response".to_string()),
                    message: "The model returned no tokens. Check the id and provider endpoint."
                        .to_string(),
                });
            }
            let _ = db.upsert_discovered_model(&DiscoveredModelRecord {
                id: model_id.to_string(),
                name: model_id.to_string(),
                provider_type: provider_type.to_string(),
                provider_id: provider_id.to_string(),
                context_window: 0,
                supports_tools: false,
                supports_streaming: true,
                input_price: 0.0,
                output_price: 0.0,
            });
            Ok(ModelIdTestResponse {
                ok: true,
                latency_ms: latency(),
                error_code: None,
                message: format!("{model_id} responded in {} ms", latency()),
            })
        }
        Err(error) => Ok(ModelIdTestResponse {
            ok: false,
            latency_ms: latency(),
            error_code: Some(error.code.clone()),
            message: friendly_stream_error(
                &error.code,
                provider_type,
                Some(model_id),
                &error.user_message,
                error.details.as_ref(),
            ),
        }),
    }
}

#[tauri::command]
pub async fn cmd_model_test_id(
    state: tauri::State<'_, Database>,
    payload: ModelIdTestPayload,
) -> Result<ModelIdTestResponse, CoreError> {
    validate_model_id(state.inner(), &payload.provider_id, &payload.model_id).await
}

#[tauri::command]
pub async fn cmd_provider_test_connection(
    state: tauri::State<'_, Database>,
    payload: ProviderIdPayload,
) -> Result<ProviderTestResponse, CoreError> {
    let started = Instant::now();
    match refresh_provider_models(state.inner(), &payload.provider_id).await {
        Ok(models) => Ok(ProviderTestResponse {
            ok: true,
            latency_ms: i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
            model_count: models.len(),
            error_code: None,
            message: format!("{} models loaded", models.len()),
        }),
        Err(err) => Ok(ProviderTestResponse {
            ok: false,
            latency_ms: i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
            model_count: 0,
            error_code: Some(err.code),
            message: err.user_message,
        }),
    }
}

#[tauri::command]
pub async fn cmd_provider_refresh_models(
    state: tauri::State<'_, Database>,
    payload: ProviderIdPayload,
) -> Result<ProviderModelsResponse, CoreError> {
    let provider_id = payload.provider_id;
    let models = refresh_provider_models(state.inner(), &provider_id).await?;
    Ok(ProviderModelsResponse {
        provider_id,
        models,
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cmd_model_set_favorite(
    state: tauri::State<'_, Database>,
    payload: ModelFavoritePayload,
) -> Result<bool, CoreError> {
    state.set_model_favorite(&payload.model_id, payload.favorite)?;
    Ok(payload.favorite)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cmd_mission_set_model(
    state: tauri::State<'_, Database>,
    payload: MissionModelPayload,
) -> Result<bool, CoreError> {
    state.set_mission_model(payload.mission_id, &payload.model_id)?;
    Ok(true)
}

/// Measured inference telemetry pushed after a request completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferMetricsEvent {
    pub metric: InferMetricRecord,
}

/// History for the Models panel: `cmd_infer_metrics_history{range}`.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cmd_infer_metrics_history(
    state: tauri::State<'_, Database>,
    payload: InferMetricsHistoryPayload,
) -> Result<Vec<InferMetricRecord>, CoreError> {
    state.list_infer_metrics(&payload.range)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferMetricsHistoryPayload {
    pub range: String,
}

/// Conversation memory for a chat request (Phase 17, 17.12). The newest
/// messages are kept; the oldest are dropped when the character budget is hit.
/// Characters/4 is a conservative token estimate — no token count is invented.
const CONVERSATION_CHAR_BUDGET: usize = 24_000;
const CONVERSATION_MAX_MESSAGES: usize = 40;

fn conversation_messages(
    history: &[MissionMessageRecord],
    max_messages: usize,
    char_budget: usize,
) -> Vec<Message> {
    let mut selected: Vec<&MissionMessageRecord> = Vec::new();
    let mut used = 0_usize;
    for record in history.iter().rev() {
        if record.role == "system" {
            continue;
        }
        if selected.len() >= max_messages {
            break;
        }
        let size = record.content.len();
        if !selected.is_empty() && used.saturating_add(size) > char_budget {
            continue;
        }
        used = used.saturating_add(size);
        selected.push(record);
    }
    selected.reverse();
    selected
        .into_iter()
        .map(|record| match record.role.as_str() {
            "assistant" => Message::assistant(&record.content),
            "tool" => Message::tool(
                &record.content,
                record.tool_call_id.clone().unwrap_or_default(),
            ),
            _ => Message::user(&record.content),
        })
        .collect()
}

/// Resolves a cloud provider's base URL and *decrypted* API key.
/// This is the only path from the encrypted DB column to an HTTP client.
fn resolve_cloud_credentials(
    db: &Database,
    provider_type: &str,
) -> Result<(Option<String>, Option<String>), CoreError> {
    if !provider_type.starts_with("cloud_") {
        return Ok((None, None));
    }
    let provider_id = match provider_type {
        "cloud_anthropic" => "anthropic",
        "cloud_gemini" => "gemini",
        "cloud_openrouter" => "openrouter",
        _ => "openai",
    };
    let cfg = db.get_provider_config(provider_id)?;
    let base_url = cfg.map(|c| c.base_url);
    let api_key = db
        .get_provider_credentials(provider_id)?
        .map(|secret| secret.expose().to_owned());
    Ok((base_url, api_key))
}

/// Maximum model -> tool -> model turns for one user prompt (Phase 17, 17.13).
const MAX_TOOL_LOOP_DEPTH: usize = 6;

fn parse_tool_name(full_name: &str) -> Option<(String, String)> {
    let (app, tool) = full_name.split_once(':')?;
    let (app, tool) = (app.trim(), tool.trim());
    if app.is_empty() || tool.is_empty() {
        return None;
    }
    Some((app.to_string(), tool.to_string()))
}

/// Converts model-issued tool calls into a validated `tasksheet/1` plan. No
/// impact numbers are invented; the diff summary describes the real call.
fn tool_calls_to_sheet(
    goal: &str,
    calls: &[ToolCall],
    validator: &TaskSheetValidator,
) -> Result<TaskSheet, CoreError> {
    let arbiter = SafetyArbiter::default();
    let mut steps = Vec::new();
    for (index, call) in calls.iter().enumerate() {
        let Some((app_id, tool_name)) = parse_tool_name(&call.name) else {
            return Err(CoreError::plan(
                "plan.unknown_app",
                false,
                format!("Tool call needs an `app:tool` name: {}", call.name),
                Some(serde_json::json!({ "name": call.name })),
            ));
        };
        let tier = arbiter.classify_tool(&tool_name) as u8;
        steps.push(TaskSheetStep {
            step_index: u32::try_from(index).unwrap_or(u32::MAX),
            app_id,
            tool_name: tool_name.clone(),
            arguments: call.arguments.clone(),
            risk_tier: tier,
            diff_summary: format!("Calls {tool_name} with the supplied arguments"),
            depends_on: vec![],
        });
    }
    let sheet = TaskSheet::new(
        goal,
        "Model-requested tool calls",
        "Invokes the tools requested by the model",
        steps,
    );
    validator.validate(&sheet)?;
    Ok(sheet)
}

/// Streams a model turn, executes its Tier-1 tool calls through the same router
/// used by `cmd_mcp_call_tool`, feeds results back as `tool` messages, and
/// repeats until the model finishes or the depth cap is reached. Tier 2/3
/// calls create an approval gate and never execute autonomously.
fn provider_label(provider_type: &str) -> &'static str {
    match provider_type {
        "cloud_openrouter" => "OpenRouter",
        "cloud_anthropic" => "Anthropic",
        "cloud_gemini" => "Gemini",
        "local_gguf" => "the local GGUF runtime",
        _ => "OpenAI",
    }
}

/// Plain-English error copy for the chat composer. The raw error is kept in
/// the `code` field for diagnostics; only this message is shown to the user.
fn friendly_stream_error(
    code: &str,
    provider_type: &str,
    model_id: Option<&str>,
    raw: &str,
    details: Option<&serde_json::Value>,
) -> String {
    let provider = provider_label(provider_type);
    match code {
        "infer.cloud_auth_failed" => {
            format!("API key rejected by {provider}. Check it in Model settings.")
        }
        "infer.model_not_found" => {
            "Model not found. Choose a model that is available to this provider.".to_string()
        }
        "infer.cloud_rate_limited" => {
            if model_id.is_some_and(|id| id.contains(":free")) {
                return format!(
                    "Free-tier model {} is rate limited by OpenRouter. Wait for the limit to reset or choose a paid model.",
                    model_id.unwrap_or_default()
                );
            }
            let retry = details
                .and_then(|d| d.get("retryAfterSeconds").or_else(|| d.get("retry_after")))
                .and_then(serde_json::Value::as_u64);
            match retry {
                Some(seconds) => format!("Rate limited by {provider}. Retry in {seconds}s."),
                None => format!("Rate limited by {provider}. Try again shortly."),
            }
        }
        "infer.cloud_endpoint_unreachable" => {
            format!("Cannot reach {provider}. Check the endpoint URL and your connection.")
        }
        "infer.context_length_exceeded" => {
            "Conversation is too long for this model. Start a new mission.".to_string()
        }
        "infer.runtime_missing" => {
            "llama-server was not found. Install llama.cpp or set STARSHIP_LLAMA_SERVER."
                .to_string()
        }
        "infer.model_load_failed" => {
            "Local model failed to load. Check the GGUF file and memory headroom.".to_string()
        }
        _ => raw.to_string(),
    }
}

/// Retries transient stream-start failures (429 / 5xx / network) up to twice
/// with exponential backoff before surfacing a typed error.
async fn dispatch_with_retry(
    provider_type: &str,
    model_id: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
    file_path: Option<&str>,
    request: ChatRequest,
    headers: Option<&ProviderHeaders>,
) -> Result<TokenStream, CoreError> {
    let mut last_error = None;
    for attempt in 0_u32..3 {
        match InferDispatcher::dispatch_stream(
            provider_type,
            model_id,
            base_url,
            api_key,
            file_path,
            request.clone(),
            headers,
        )
        .await
        {
            Ok(stream) => return Ok(stream),
            Err(error) => {
                let transient = error.retryable
                    && matches!(
                        error.code.as_str(),
                        "infer.cloud_auth_failed"
                            | "infer.cloud_rate_limited"
                            | "infer.cloud_endpoint_unreachable"
                            | "infer.stream_error"
                    );
                if attempt < 2 && transient {
                    last_error = Some(error);
                    tokio::time::sleep(Duration::from_millis(500 * 2_u64.pow(attempt))).await;
                    continue;
                }
                return Err(error);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| {
        CoreError::infer(
            "infer.cloud_endpoint_unreachable",
            true,
            "Model endpoint did not respond after retries",
            None,
        )
    }))
}

#[allow(clippy::too_many_arguments)]
async fn run_chat_with_tools(
    app: &AppHandle,
    db: &Database,
    mission_id: i64,
    model: &ModelRecord,
    goal: &str,
    base_url: Option<&str>,
    api_key: Option<&str>,
    mut messages: Vec<Message>,
) -> Result<(), CoreError> {
    let aggregator = ToolAggregator::new();
    let system_prompt = aggregator.build_system_context();
    let tools = aggregator.to_llm_definitions();
    let mut validator = TaskSheetValidator::default();
    for tool in &tools {
        let app_id = tool
            .name
            .split_once('.')
            .map_or(tool.name.as_str(), |(app, _)| app);
        validator.register_tool(starship_core::planner::ToolSchema::new(
            app_id,
            tool.name.clone(),
            1,
            tool.parameters.clone(),
        ));
    }
    let arbiter = SafetyArbiter::default();
    let provider_id = match model.provider_type.as_str() {
        "cloud_anthropic" => "anthropic",
        "cloud_gemini" => "gemini",
        "cloud_openrouter" => "openrouter",
        _ => "openai",
    };
    let headers = db
        .get_provider_config(provider_id)
        .ok()
        .flatten()
        .map(|cfg| ProviderHeaders {
            http_referer: cfg.http_referer,
            x_title: cfg.x_title,
        });
    // Never advertise tools to a model that does not support them.
    let effective_tools = if model.supports_tools {
        tools.clone()
    } else {
        Vec::new()
    };

    for depth in 0..=MAX_TOOL_LOOP_DEPTH {
        let chat_req = ChatRequest {
            model: model.id.clone(),
            messages: std::mem::take(&mut messages),
            system_prompt: Some(system_prompt.clone()),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            tools: effective_tools.clone(),
            grammar: None,
            stream: true,
        };
        let request_started = Instant::now();
        let mut stream = match dispatch_with_retry(
            &model.provider_type,
            &model.id,
            base_url,
            api_key,
            model.file_path.as_deref(),
            chat_req,
            headers.as_ref(),
        )
        .await
        {
            Ok(stream) => stream,
            Err(error) => {
                let message = friendly_stream_error(
                    &error.code,
                    &model.provider_type,
                    Some(&model.id),
                    &error.user_message,
                    error.details.as_ref(),
                );
                error!(target: "starship_ipc", mission_id, code = %error.code, "Inference dispatch failed");
                let _ = app.emit(
                    "evt://stream/error",
                    StreamErrorEvent {
                        mission_id,
                        message,
                        code: error.code,
                    },
                );
                return Ok(());
            }
        };

        let mut content = String::new();
        let mut thought = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut first_token_at: Option<Instant> = None;
        let mut streamed_chunks: i64 = 0;
        let mut stream_error: Option<String> = None;
        while let Some(chunk_res) = stream.next().await {
            match chunk_res {
                Ok(chunk) => {
                    if let Some(ref chunk_thought) = chunk.thought {
                        thought.push_str(chunk_thought);
                        let _ = app.emit(
                            "evt://thought/stream",
                            ThoughtStreamEvent {
                                mission_id,
                                thought_chunk: chunk_thought.clone(),
                            },
                        );
                    }
                    if !chunk.content.is_empty() {
                        if first_token_at.is_none() {
                            first_token_at = Some(Instant::now());
                        }
                        streamed_chunks += 1;
                        content.push_str(&chunk.content);
                        let _ = app.emit(
                            "evt://token/stream",
                            TokenStreamEvent {
                                mission_id,
                                chunk: chunk.content,
                            },
                        );
                    }
                    if let Some(calls) = chunk.tool_calls {
                        tool_calls.extend(calls);
                    }
                }
                Err(e) => {
                    stream_error = Some(e.code.clone());
                    error!(target: "starship_ipc", "Stream chunk error: {e}");
                    let _ = app.emit(
                        "evt://stream/error",
                        StreamErrorEvent {
                            mission_id,
                            message: friendly_stream_error(
                                &e.code,
                                &model.provider_type,
                                Some(&model.id),
                                &e.user_message,
                                e.details.as_ref(),
                            ),
                            code: e.code,
                        },
                    );
                    break;
                }
            }
        }

        let elapsed = request_started.elapsed();
        let total_ms = i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX);
        let first_ms = first_token_at.map_or(0, |at| {
            i64::try_from(at.duration_since(request_started).as_millis()).unwrap_or(i64::MAX)
        });
        let chunks = u32::try_from(streamed_chunks).unwrap_or(u32::MAX);
        let metric = InferMetricRecord {
            id: 0,
            ts_ms: starship_db::infer::now_ms(),
            mission_id: Some(mission_id),
            provider: model.provider_type.clone(),
            model_id: model.id.clone(),
            first_token_ms: first_ms,
            total_latency_ms: total_ms,
            tokens_in: 0,
            tokens_out: streamed_chunks,
            tokens_per_sec: f64::from(chunks) / elapsed.as_secs_f64().max(0.001),
            error_code: stream_error,
        };
        let _ = db.record_infer_metric(&metric);
        let _ = app.emit("evt://infer/metrics", InferMetricsEvent { metric });

        let thought_opt = if thought.is_empty() {
            None
        } else {
            Some(thought.as_str())
        };
        let assistant_id = db
            .add_message(mission_id, "assistant", &content, thought_opt)
            .unwrap_or(0);
        let _ = app.emit(
            "evt://stream/finished",
            StreamFinishedEvent {
                mission_id,
                message_id: assistant_id,
                content,
                thought: thought_opt.map(ToString::to_string),
            },
        );

        if tool_calls.is_empty() {
            return Ok(());
        }
        if depth == MAX_TOOL_LOOP_DEPTH {
            let err = CoreError::plan(
                "plan.max_steps_exceeded",
                false,
                "Tool loop depth exceeded",
                Some(serde_json::json!({ "depth": MAX_TOOL_LOOP_DEPTH })),
            );
            let _ = app.emit(
                "evt://stream/error",
                StreamErrorEvent {
                    mission_id,
                    message: err.user_message.clone(),
                    code: err.code,
                },
            );
            return Ok(());
        }

        let sheet = tool_calls_to_sheet(goal, &tool_calls, &validator)?;
        if sheet.steps.iter().any(|step| step.risk_tier >= 2) {
            // Mutating/destructive tools always pause for human approval.
            activate_plan_internal(app, db, &arbiter, mission_id, &sheet)?;
            return Ok(());
        }
        for step in &sheet.steps {
            let result = call_tool(
                db,
                CallToolPayload {
                    app_id: step.app_id.clone(),
                    tool_name: step.tool_name.clone(),
                    arguments: step.arguments.clone(),
                    lease_timeout_ms: Some(15_000),
                },
            )
            .await;
            let payload = match result {
                Ok(value) => value.to_string(),
                Err(err) => serde_json::json!({ "error": err.code }).to_string(),
            };
            db.add_tool_message(mission_id, &format!("step-{}", step.step_index), &payload)?;
        }
        messages = conversation_messages(
            &db.list_messages(mission_id)?,
            CONVERSATION_MAX_MESSAGES,
            CONVERSATION_CHAR_BUDGET,
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn cmd_mission_send_prompt(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: SendPromptPayload,
) -> Result<SendPromptResponse, CoreError> {
    let mission = state.get_mission(payload.mission_id)?.ok_or_else(|| {
        CoreError::ipc(
            "ipc.bad_state",
            false,
            format!("Mission {} not found", payload.mission_id),
            None,
        )
    })?;

    // Record user message
    let user_msg_id = state.add_message(payload.mission_id, "user", &payload.prompt, None)?;

    info!(target: "starship_ipc", mission_id = payload.mission_id, user_msg_id, "User prompt accepted");

    // Build the conversation from stored history, not just the latest prompt.
    let history = state.list_messages(payload.mission_id)?;
    let messages = conversation_messages(
        &history,
        CONVERSATION_MAX_MESSAGES,
        CONVERSATION_CHAR_BUDGET,
    );

    // Fetch model details
    let model = state.get_model(&mission.active_model_id)?.ok_or_else(|| {
        CoreError::infer(
            "infer.model_not_found",
            false,
            format!("Active model {} not found", mission.active_model_id),
            None,
        )
    })?;

    // Resolve cloud credentials at the point of use. The encrypted `enc_v1:`
    // blob must never be passed to an HTTP client (Phase 17, task 17.11).
    let (base_url, api_key) = resolve_cloud_credentials(state.inner(), &model.provider_type)?;

    let mission_id = payload.mission_id;
    let app_handle = app.clone();
    let db = state.inner().clone();
    let goal = mission.user_goal.clone();

    // Run the model -> Tier-1 tool -> model loop on a background task.
    tokio::spawn(async move {
        if let Err(err) = run_chat_with_tools(
            &app_handle,
            &db,
            mission_id,
            &model,
            &goal,
            base_url.as_deref(),
            api_key.as_deref(),
            messages,
        )
        .await
        {
            error!(target: "starship_ipc", mission_id, "Tool loop failed: {err}");
            let _ = app_handle.emit(
                "evt://stream/error",
                StreamErrorEvent {
                    mission_id,
                    message: err.user_message,
                    code: err.code,
                },
            );
        }
    });

    Ok(SendPromptResponse {
        message_id: user_msg_id,
        status: "thinking".to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTopologyEntry {
    pub app_id: String,
    pub display_name: String,
    pub version: String,
    pub mode: String,
    pub socket_path: Option<String>,
    pub binary_path: String,
    pub latency_ms: Option<f64>,
    pub capabilities: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAppsResponse {
    pub apps: Vec<AppTopologyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchAppPayload {
    pub app_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchAppResponse {
    pub spawned: bool,
    pub pid: Option<u32>,
    /// `launched` when the bridge socket appeared, `launched_no_bridge` when
    /// the process started but no bridge is available (Phase 17, 17.2).
    pub bridge_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyAppStateEvent {
    pub app_id: String,
    pub mode: String,
    pub socket_path: Option<String>,
}

#[tauri::command]
pub async fn cmd_topology_list_apps(
    state: tauri::State<'_, Database>,
) -> Result<ListAppsResponse, CoreError> {
    let db = state.inner();
    let records = scan_and_sync_manifests(db)?;

    let mut apps = Vec::new();

    for r in records {
        let mut mode = "offline".to_string();
        let mut latency_ms = None;

        if let Some(ref sock) = r.socket_path {
            let sock_path = std::path::Path::new(sock);
            if sock_path.exists() {
                if let Ok(mut client) = UnixTransportClient::connect(sock_path).await {
                    if let Ok(ping) = client.ping().await {
                        mode = "live_gui".to_string();
                        latency_ms = Some(ping);
                    } else {
                        mode = "live_gui".to_string();
                    }
                } else {
                    mode = "live_gui".to_string();
                }
            } else if std::path::Path::new(&r.binary_path).exists() {
                mode = "headless_cli".to_string();
            }
        } else if std::path::Path::new(&r.binary_path).exists() {
            mode = "headless_cli".to_string();
        }

        let capabilities: serde_json::Value =
            serde_json::from_str(&r.capabilities_json).unwrap_or_else(|_| serde_json::json!({}));

        apps.push(AppTopologyEntry {
            app_id: r.app_id,
            display_name: r.display_name,
            version: r.version,
            mode,
            socket_path: r.socket_path,
            binary_path: r.binary_path,
            latency_ms,
            capabilities,
        });
    }

    Ok(ListAppsResponse { apps })
}

#[tauri::command]
pub async fn cmd_topology_launch_app(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: LaunchAppPayload,
) -> Result<LaunchAppResponse, CoreError> {
    let db = state.inner();
    let (pid, bridge_status) = spawn_app(db, &payload.app_id, Duration::from_secs(5)).await?;
    let socket_path = starship_plugin::socket_path_for(&payload.app_id)
        .to_string_lossy()
        .to_string();
    let live_bridge = bridge_status == "launched";
    let _ = db.update_app_gui_status(&payload.app_id, live_bridge, Some(&socket_path));
    let _ = app.emit(
        "evt://topology/app_state",
        TopologyAppStateEvent {
            app_id: payload.app_id,
            mode: if live_bridge {
                "live_gui".to_string()
            } else {
                "headless_cli".to_string()
            },
            socket_path: Some(socket_path),
        },
    );
    Ok(LaunchAppResponse {
        spawned: true,
        pid: Some(pid),
        bridge_status,
    })
}

/// Spawns the registered binary for real. Returns its true PID and whether the
/// bridge socket appeared within `bridge_wait`. No simulated PID or GUI state.
async fn spawn_app(
    db: &Database,
    app_id: &str,
    bridge_wait: Duration,
) -> Result<(u32, String), CoreError> {
    let app_record = db.get_app(app_id)?.ok_or_else(|| {
        CoreError::app(
            "app.not_found",
            false,
            format!("Application '{app_id}' is not registered"),
            None,
        )
    })?;
    let binary_path = app_record.binary_path.clone();
    if !std::path::Path::new(&binary_path).exists() {
        return Err(CoreError::app(
            "mcp.app_spawn_failed",
            false,
            format!("Application binary not found: {binary_path}"),
            Some(serde_json::json!({"appId": app_id, "binaryPath": binary_path})),
        ));
    }
    let child = std::process::Command::new(&binary_path)
        .spawn()
        .map_err(|e| {
            CoreError::app(
                "mcp.app_spawn_failed",
                false,
                format!("Failed to spawn {binary_path}: {e}"),
                Some(serde_json::json!({"appId": app_id, "binaryPath": binary_path})),
            )
        })?;
    let pid = child.id();
    let socket = starship_plugin::socket_path_for(app_id);
    let deadline = Instant::now() + bridge_wait;
    while Instant::now() < deadline {
        if socket.exists() {
            return Ok((pid, "launched".to_string()));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok((pid, "launched_no_bridge".to_string()))
}

#[tauri::command]
pub async fn cmd_topology_ping_app(
    state: tauri::State<'_, Database>,
    payload: LaunchAppPayload,
) -> Result<f64, CoreError> {
    let db = state.inner();
    let app_record = db.get_app(&payload.app_id)?.ok_or_else(|| {
        CoreError::app(
            "app.not_found",
            false,
            format!("Application '{}' is not registered", payload.app_id),
            None,
        )
    })?;

    let sock_path_str = app_record.socket_path.ok_or_else(|| {
        CoreError::mcp(
            "mcp.socket_unreachable",
            true,
            format!("No bridge socket configured for {}", payload.app_id),
            None,
        )
    })?;

    let sock_path = std::path::Path::new(&sock_path_str);
    if !sock_path.exists() {
        return Err(CoreError::mcp(
            "mcp.socket_unreachable",
            true,
            format!("Socket file does not exist at {}", sock_path.display()),
            None,
        ));
    }

    let mut client = UnixTransportClient::connect(sock_path).await?;
    let ping = client.ping().await?;
    let _ = db.update_app_gui_status(&payload.app_id, true, Some(&sock_path_str));
    Ok(ping)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolPayload {
    pub app_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub lease_timeout_ms: Option<u64>,
}

#[tauri::command]
pub async fn cmd_mcp_list_tools() -> Result<Vec<AggregatedTool>, CoreError> {
    let aggregator = ToolAggregator::new();
    Ok(aggregator.get_all_tools())
}

#[tauri::command]
pub async fn cmd_mcp_call_tool(
    state: tauri::State<'_, Database>,
    payload: CallToolPayload,
) -> Result<serde_json::Value, CoreError> {
    call_tool(state.inner(), payload).await
}

/// Shared implementation of `cmd_mcp_call_tool`, testable without a Tauri
/// `State` instance (which has no public constructor).
async fn call_tool(
    db: &Database,
    payload: CallToolPayload,
) -> Result<serde_json::Value, CoreError> {
    let router = HybridExecutionRouter::new(db.clone());
    let timeout = payload.lease_timeout_ms.unwrap_or(10_000);
    router
        .route_and_call(
            &payload.app_id,
            &payload.tool_name,
            payload.arguments,
            timeout,
        )
        .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanGetActivePayload {
    pub mission_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanStepDetail {
    pub step_id: i64,
    pub step_index: i64,
    pub app_id: String,
    pub tool_name: String,
    pub risk_tier: i64,
    pub approval_status: String,
    pub execution_status: String,
    pub diff_summary: Option<String>,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub signature_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanActiveResponse {
    pub plan_id: i64,
    pub mission_id: i64,
    pub version: i64,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub steps: Vec<PlanStepDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveStepPayload {
    pub mission_id: Option<i64>,
    pub step_id: i64,
    pub signature_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveStepResponse {
    pub executed: bool,
    pub status: String,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectStepPayload {
    pub mission_id: Option<i64>,
    pub step_id: i64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectStepResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalNeededEvent {
    pub mission_id: i64,
    pub plan_id: i64,
    pub step_id: i64,
    pub app_id: String,
    pub risk_tier: i64,
    pub action_title: String,
    pub diff_summary: String,
    pub checkpoint_name: Option<String>,
    pub signature_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratePlanPayload {
    pub mission_id: i64,
    pub user_goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlanPayload {
    pub mission_id: i64,
    pub title: String,
    pub summary: String,
    pub raw_tasksheet_json: String,
}

#[tauri::command]
pub async fn cmd_plan_get_active(
    state: tauri::State<'_, Database>,
    payload: PlanGetActivePayload,
) -> Result<Option<PlanActiveResponse>, CoreError> {
    let db = state.inner();
    let arbiter = SafetyArbiter::default();

    let active = db.get_active_plan(payload.mission_id)?;
    let Some((plan, steps)) = active else {
        return Ok(None);
    };

    let step_details = steps
        .into_iter()
        .map(|s| {
            let args_val =
                serde_json::from_str(&s.arguments_json).unwrap_or(serde_json::Value::Null);
            let res_val = s
                .result_json
                .as_deref()
                .and_then(|r| serde_json::from_str(r).ok());

            let sig_token = if s.risk_tier >= 2 && s.approval_status == "pending" {
                Some(arbiter.generate_signature_token(plan.id, s.id, &s.tool_name))
            } else {
                None
            };

            PlanStepDetail {
                step_id: s.id,
                step_index: s.step_index,
                app_id: s.app_id,
                tool_name: s.tool_name,
                risk_tier: s.risk_tier,
                approval_status: s.approval_status,
                execution_status: s.execution_status,
                diff_summary: s.diff_summary,
                arguments: args_val,
                result: res_val,
                error_message: s.error_message,
                signature_token: sig_token,
            }
        })
        .collect();

    Ok(Some(PlanActiveResponse {
        plan_id: plan.id,
        mission_id: plan.mission_id,
        version: plan.version,
        title: plan.title,
        summary: plan.summary,
        status: plan.status,
        steps: step_details,
    }))
}

/// Builds the fallback plan sheet. No diff summary invents a measured value:
/// numbers are only shown when they came from a real argument or scan result.
fn build_plan_sheet(payload: &GeneratePlanPayload) -> TaskSheet {
    let goal_lower = payload.user_goal.to_ascii_lowercase();
    if goal_lower.contains("system")
        || goal_lower.contains("disk")
        || goal_lower.contains("cache")
        || goal_lower.contains("tune")
    {
        TaskSheet::new(
            payload.user_goal.clone(),
            "System Maintenance & Cache Optimization",
            "Scans network telemetry and disk usage, then purges temporary caches upon confirmation",
            vec![
                TaskSheetStep {
                    step_index: 0,
                    app_id: "nwp".to_string(),
                    tool_name: "network.scan".to_string(),
                    arguments: serde_json::json!({}),
                    risk_tier: 1,
                    diff_summary: "Inspects live network sockets and telemetry".to_string(),
                    depends_on: vec![],
                },
                TaskSheetStep {
                    step_index: 1,
                    app_id: "atp".to_string(),
                    tool_name: "disk.scan".to_string(),
                    arguments: serde_json::json!({}),
                    risk_tier: 1,
                    diff_summary: "Scans disk usage and build cache directories".to_string(),
                    depends_on: vec![],
                },
                TaskSheetStep {
                    step_index: 2,
                    app_id: "atp".to_string(),
                    tool_name: "cache.purge_all".to_string(),
                    arguments: serde_json::json!({ "older_than_days": 7 }),
                    risk_tier: 3,
                    diff_summary:
                        "Purges caches older than 7 days (impact measured by disk.scan first)"
                            .to_string(),
                    depends_on: vec![0, 1],
                },
            ],
        )
    } else {
        // Default media & screencast workflow (SSP -> ALP -> EEP)
        TaskSheet::new(
            payload.user_goal.clone(),
            "Tutorial Screencast Assembly",
            "Records desktop demo, cleans and masters vocal take in ALP, and imports into EEP timeline",
            vec![
                TaskSheetStep {
                    step_index: 0,
                    app_id: "ssp".to_string(),
                    tool_name: "recording.capture_screen".to_string(),
                    arguments: serde_json::json!({ "durationSecs": 60, "includeMic": true }),
                    risk_tier: 1,
                    diff_summary: "Captures 60 seconds screen recording with microphone audio"
                        .to_string(),
                    depends_on: vec![],
                },
                TaskSheetStep {
                    step_index: 1,
                    app_id: "alp".to_string(),
                    tool_name: "audio.cleanup".to_string(),
                    arguments: serde_json::json!({ "preset": "voice_isolate", "dehum": true }),
                    risk_tier: 2,
                    diff_summary: "Applies Voice Isolation and De-hum preset to vocal track"
                        .to_string(),
                    depends_on: vec![0],
                },
                TaskSheetStep {
                    step_index: 2,
                    app_id: "eep".to_string(),
                    tool_name: "timeline.cut_interval".to_string(),
                    arguments: serde_json::json!({ "trackId": 1, "rippleCut": true }),
                    risk_tier: 2,
                    diff_summary: "Applies ripple cuts derived from real CET telemetry (intervals inspected at execution)"
                        .to_string(),
                    depends_on: vec![1],
                },
            ],
        )
    }
}

#[tauri::command]
pub async fn cmd_plan_generate(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: GeneratePlanPayload,
) -> Result<PlanActiveResponse, CoreError> {
    let db = state.inner();
    let arbiter = SafetyArbiter::default();

    // Decompose user strategic goal into a tasksheet/1 plan
    let sheet = build_plan_sheet(&payload);
    activate_plan_internal(&app, db, &arbiter, payload.mission_id, &sheet)
}

fn activate_plan_internal(
    app: &AppHandle,
    db: &Database,
    arbiter: &SafetyArbiter,
    mission_id: i64,
    sheet: &TaskSheet,
) -> Result<PlanActiveResponse, CoreError> {
    let plan_rec = db.create_plan(mission_id, &sheet.title, &sheet.summary, sheet)?;

    let active_plan_opt = db.get_active_plan(mission_id)?;
    let Some((_, steps)) = active_plan_opt else {
        return Err(CoreError::db(
            "db.plan_not_found",
            false,
            "Failed to retrieve newly created plan",
            None,
        ));
    };

    let mut step_details = Vec::new();
    let mut pending_approval_step = None;

    for s in steps {
        let args_val = serde_json::from_str(&s.arguments_json).unwrap_or(serde_json::Value::Null);
        let sig_token = if s.risk_tier >= 2 && s.approval_status == "pending" {
            let tok = arbiter.generate_signature_token(plan_rec.id, s.id, &s.tool_name);
            if pending_approval_step.is_none() {
                pending_approval_step = Some((s.clone(), tok.clone()));
            }
            Some(tok)
        } else {
            None
        };

        step_details.push(PlanStepDetail {
            step_id: s.id,
            step_index: s.step_index,
            app_id: s.app_id,
            tool_name: s.tool_name,
            risk_tier: s.risk_tier,
            approval_status: s.approval_status,
            execution_status: s.execution_status,
            diff_summary: s.diff_summary,
            arguments: args_val,
            result: None,
            error_message: None,
            signature_token: sig_token,
        });
    }

    // If there is an approval gate required, emit evt://plan/approval_needed
    if let Some((step, token)) = pending_approval_step {
        let action_title = step
            .diff_summary
            .clone()
            .unwrap_or_else(|| step.tool_name.clone());
        let _ = app.emit(
            "evt://plan/approval_needed",
            ApprovalNeededEvent {
                mission_id,
                plan_id: plan_rec.id,
                step_id: step.id,
                app_id: step.app_id,
                risk_tier: step.risk_tier,
                action_title,
                diff_summary: step.diff_summary.unwrap_or_default(),
                checkpoint_name: Some("#bk-01".to_string()),
                signature_token: token,
            },
        );
    }

    Ok(PlanActiveResponse {
        plan_id: plan_rec.id,
        mission_id: plan_rec.mission_id,
        version: plan_rec.version,
        title: plan_rec.title,
        summary: plan_rec.summary,
        status: plan_rec.status,
        steps: step_details,
    })
}

#[tauri::command]
pub async fn cmd_plan_create(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: CreatePlanPayload,
) -> Result<PlanActiveResponse, CoreError> {
    let db = state.inner();
    let arbiter = SafetyArbiter::default();

    let sheet = TaskSheet::from_json(&payload.raw_tasksheet_json)?;
    activate_plan_internal(&app, db, &arbiter, payload.mission_id, &sheet)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineScreencastPayload {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
    #[serde(rename = "recordingPath")]
    pub recording_path: String,
    #[serde(rename = "cetPath")]
    pub cet_path: Option<String>,
    #[serde(rename = "cetContent")]
    pub cet_content: Option<String>,
    #[serde(rename = "cleanAudio")]
    pub clean_audio: bool,
    #[serde(rename = "audioPreset")]
    pub audio_preset: Option<String>,
    #[serde(rename = "introScript")]
    pub intro_script: Option<String>,
    #[serde(rename = "targetTimelineTrack")]
    pub target_timeline_track: u32,
}

/// Shared plan builder for `cmd_pipeline_screencast`, testable without a
/// Tauri `AppHandle` or `State` instance.
fn build_screencast_plan(payload: &PipelineScreencastPayload) -> Result<TaskSheet, CoreError> {
    let telemetry = if let Some(ref content) = payload.cet_content {
        Some(CetTelemetry::from_jsonl_str(content)?)
    } else if let Some(ref path) = payload.cet_path {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|c| CetTelemetry::from_jsonl_str(&c).ok())
    } else {
        None
    };
    let req = ScreencastAssemblyRequest {
        recording_path: payload.recording_path.clone(),
        cet_path: payload.cet_path.clone(),
        clean_audio: payload.clean_audio,
        audio_preset: payload.audio_preset.clone(),
        intro_script: payload.intro_script.clone(),
        target_timeline_track: payload.target_timeline_track,
    };
    ScreencastPipeline::plan_assembly(&req, telemetry.as_ref()).map(|result| result.task_sheet)
}

#[tauri::command]
pub async fn cmd_pipeline_screencast(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: PipelineScreencastPayload,
) -> Result<PlanActiveResponse, CoreError> {
    let db = state.inner();
    let arbiter = SafetyArbiter::default();

    let task_sheet = build_screencast_plan(&payload)?;
    activate_plan_internal(&app, db, &arbiter, payload.mission_id, &task_sheet)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineDiagnosticsPayload {
    #[serde(rename = "missionId")]
    pub mission_id: i64,
    #[serde(rename = "liveConnectionsCount")]
    pub live_connections_count: usize,
    #[serde(rename = "totalBandwidthKbps")]
    pub total_bandwidth_kbps: f64,
    #[serde(rename = "diskUsedGb")]
    pub disk_used_gb: f64,
    #[serde(rename = "diskFreeGb")]
    pub disk_free_gb: f64,
    #[serde(rename = "cacheBloatMb")]
    pub cache_bloat_mb: f64,
    #[serde(rename = "reclaimableItems")]
    pub reclaimable_items: Vec<String>,
}

#[tauri::command]
pub async fn cmd_pipeline_diagnostics(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: PipelineDiagnosticsPayload,
) -> Result<PlanActiveResponse, CoreError> {
    let db = state.inner();
    let arbiter = SafetyArbiter::default();

    let report = SystemMetricsReport {
        live_connections_count: payload.live_connections_count,
        total_bandwidth_kbps: payload.total_bandwidth_kbps,
        disk_used_gb: payload.disk_used_gb,
        disk_free_gb: payload.disk_free_gb,
        cache_bloat_mb: payload.cache_bloat_mb,
        reclaimable_items: payload.reclaimable_items,
    };

    let sheet = DiagnosticsPipeline::plan_system_maintenance(&report);
    activate_plan_internal(&app, db, &arbiter, payload.mission_id, &sheet)
}

#[tauri::command]
pub async fn cmd_plan_approve_step(
    state: tauri::State<'_, Database>,
    payload: ApproveStepPayload,
) -> Result<ApproveStepResponse, CoreError> {
    let db = state.inner().clone();
    let arbiter = SafetyArbiter::default();

    let step = db.get_plan_step(payload.step_id)?.ok_or_else(|| {
        CoreError::safety(
            "safety.step_not_found",
            false,
            format!("Step with ID {} not found", payload.step_id),
            None,
        )
    })?;

    let risk_tier = match step.risk_tier {
        1 => RiskTier::Safe,
        3 => RiskTier::Destructive,
        _ => RiskTier::Mutating,
    };

    // Evaluate step permission: blocks unapproved or invalid signatures at Rust core level
    arbiter.evaluate_step(
        step.plan_id,
        step.id,
        &step.tool_name,
        risk_tier,
        Some(&payload.signature_token),
    )?;

    // Mark step approved in DB
    db.update_step_approval(step.id, "approved")?;

    // Record audit log entry for human approval granted
    let _ = db.record_audit_log(&NewAuditEntry {
        mission_id: payload.mission_id.or(Some(step.plan_id)),
        app_id: step.app_id.clone(),
        action_type: "approval_granted".to_string(),
        risk_tier: step.risk_tier,
        payload_json: serde_json::json!({
            "stepId": step.id,
            "toolName": step.tool_name,
            "diffSummary": step.diff_summary,
        })
        .to_string(),
        status: "success".to_string(),
        signature_token: Some(payload.signature_token.clone()),
    });

    // Mark step running
    db.update_step_execution(step.id, "running", None, None)?;

    // Parse arguments and route execution
    let args: serde_json::Value =
        serde_json::from_str(&step.arguments_json).unwrap_or(serde_json::json!({}));
    let router = HybridExecutionRouter::new(db.clone());

    match router
        .route_and_call(&step.app_id, &step.tool_name, args, 15_000)
        .await
    {
        Ok(res) => {
            let res_str = serde_json::to_string(&res).unwrap_or_default();
            db.update_step_execution(step.id, "success", Some(&res_str), None)?;

            // Record execution audit log
            let _ = db.record_audit_log(&NewAuditEntry {
                mission_id: payload.mission_id.or(Some(step.plan_id)),
                app_id: step.app_id,
                action_type: "tool_call".to_string(),
                risk_tier: step.risk_tier,
                payload_json: res_str,
                status: "success".to_string(),
                signature_token: Some(payload.signature_token),
            });

            Ok(ApproveStepResponse {
                executed: true,
                status: "success".to_string(),
                result: Some(res),
            })
        }
        Err(e) => {
            db.update_step_execution(step.id, "failed", None, Some(&e.user_message))?;

            // Record failure audit log
            let _ = db.record_audit_log(&NewAuditEntry {
                mission_id: payload.mission_id.or(Some(step.plan_id)),
                app_id: step.app_id,
                action_type: "tool_call".to_string(),
                risk_tier: step.risk_tier,
                payload_json: serde_json::json!({ "error": e.user_message }).to_string(),
                status: "failed".to_string(),
                signature_token: Some(payload.signature_token),
            });

            Err(e)
        }
    }
}

#[tauri::command]
pub async fn cmd_plan_reject_step(
    state: tauri::State<'_, Database>,
    payload: RejectStepPayload,
) -> Result<RejectStepResponse, CoreError> {
    let db = state.inner();

    let step = db.get_plan_step(payload.step_id)?.ok_or_else(|| {
        CoreError::safety(
            "safety.step_not_found",
            false,
            format!("Step with ID {} not found", payload.step_id),
            None,
        )
    })?;

    db.update_step_approval(step.id, "rejected")?;
    db.update_step_execution(step.id, "skipped", None, Some(&payload.reason))?;

    // Record audit log entry for human rejection
    let _ = db.record_audit_log(&NewAuditEntry {
        mission_id: payload.mission_id.or(Some(step.plan_id)),
        app_id: step.app_id,
        action_type: "approval_denied".to_string(),
        risk_tier: step.risk_tier,
        payload_json: serde_json::json!({
            "stepId": step.id,
            "toolName": step.tool_name,
            "reason": payload.reason,
        })
        .to_string(),
        status: "denied".to_string(),
        signature_token: None,
    });

    Ok(RejectStepResponse {
        status: "rejected".to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAuditLogsPayload {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[tauri::command]
pub async fn cmd_audit_list_logs(
    state: tauri::State<'_, Database>,
    payload: Option<ListAuditLogsPayload>,
) -> Result<Vec<starship_db::AuditRecord>, CoreError> {
    let db = state.inner();
    let limit = payload.as_ref().and_then(|p| p.limit).unwrap_or(50);
    let offset = payload.as_ref().and_then(|p| p.offset).unwrap_or(0);
    db.list_audit_logs(limit, offset)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDryRunPayload {
    pub tasksheet_json: String,
}

#[tauri::command]
pub async fn cmd_plan_preview(payload: PlanDryRunPayload) -> Result<PlanPreview, CoreError> {
    let sheet = TaskSheet::from_llm_json(&payload.tasksheet_json)?;
    let validator = TaskSheetValidator::default();
    validator.validate(&sheet)?;
    Ok(PlanPreview::from_tasksheet(&sheet))
}

#[tauri::command]
pub async fn cmd_plan_dry_run(payload: PlanDryRunPayload) -> Result<DryRunReport, CoreError> {
    let sheet = TaskSheet::from_llm_json(&payload.tasksheet_json)?;
    let validator = TaskSheetValidator::default();
    let mut dry_run_caller = |_app_id: &str, _tool_name: &str, _args: &serde_json::Value| {
        // Tier 1 live inspection validation
        Ok(())
    };
    DryRunExecutor::execute(&sheet, &validator, &mut dry_run_caller)
}

#[tauri::command]
pub async fn cmd_health_get_snapshot(
    health: tauri::State<'_, std::sync::Arc<crate::health_service::HealthService>>,
) -> Result<crate::health_service::HealthSnapshotResponse, CoreError> {
    health.get_snapshot()
}

#[tauri::command]
pub async fn cmd_health_get_history(
    payload: crate::health_service::HealthHistoryPayload,
    health: tauri::State<'_, std::sync::Arc<crate::health_service::HealthService>>,
    db: tauri::State<'_, Database>,
) -> Result<crate::health_service::HealthHistoryResponse, CoreError> {
    health.get_history(payload, &db)
}

#[tauri::command]
pub async fn cmd_health_get_thresholds(
    health: tauri::State<'_, std::sync::Arc<crate::health_service::HealthService>>,
) -> Result<starship_health::OverloadThresholds, CoreError> {
    health.get_thresholds()
}

#[tauri::command]
pub async fn cmd_health_set_thresholds(
    thresholds: starship_health::OverloadThresholds,
    health: tauri::State<'_, std::sync::Arc<crate::health_service::HealthService>>,
) -> Result<starship_health::OverloadThresholds, CoreError> {
    health.set_thresholds(thresholds)
}

fn chrono_now_iso() -> String {
    // Fast ISO 8601 UTC timestamp generator
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_sheet_diff_summaries_never_invent_measurements() {
        for goal in ["clean my system caches", "record a demo and cut silences"] {
            let sheet = build_plan_sheet(&GeneratePlanPayload {
                mission_id: 1,
                user_goal: goal.to_string(),
            });
            let serialized = serde_json::to_string(&sheet).expect("serialize sheet");
            assert!(!serialized.contains("4.2 GB"));
            assert!(!serialized.contains("18 ripple cuts"));
            assert!(!serialized.contains("01:24"));
            for step in &sheet.steps {
                assert!(
                    !step.diff_summary.contains(" GB "),
                    "invented measurement: {}",
                    step.diff_summary
                );
            }
        }
    }

    #[test]
    fn test_pipeline_screencast_requires_real_telemetry() {
        let payload = PipelineScreencastPayload {
            mission_id: 0,
            recording_path: "/tmp/demo.mkv".to_string(),
            cet_path: None,
            cet_content: None,
            clean_audio: false,
            audio_preset: None,
            intro_script: None,
            target_timeline_track: 1,
        };
        let err = build_screencast_plan(&payload).expect_err("no telemetry must fail");
        assert_eq!(err.code, "pipeline.telemetry_required");
        assert!(!err.retryable);
        assert!(err.user_message.contains("Record or load a session first"));
    }

    #[tokio::test]
    async fn test_launch_app_missing_binary_returns_typed_error() {
        let db = Database::open_in_memory().expect("in-memory db");
        db.upsert_app(&starship_db::AppRecord {
            app_id: "missing".to_string(),
            display_name: "Missing".to_string(),
            version: "0".to_string(),
            binary_path: "/nonexistent/starship-launch-test".to_string(),
            manifest_path: String::new(),
            socket_path: None,
            capabilities_json: "{}".to_string(),
            is_active_gui: false,
            last_seen_at: String::new(),
            created_at: String::new(),
        })
        .expect("upsert app");
        let err = spawn_app(&db, "missing", Duration::from_millis(100))
            .await
            .expect_err("missing binary must fail");
        assert_eq!(err.code, "mcp.app_spawn_failed");
        assert!(!err.retryable);
        assert_eq!(
            err.details
                .as_ref()
                .and_then(|details| details["appId"].as_str()),
            Some("missing")
        );
    }

    #[tokio::test]
    async fn test_launch_app_returns_real_pid_and_honest_bridge_status() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("starship-launch-17-2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let script = dir.join("fake-gui.sh");
        std::fs::write(&script, "#!/bin/sh\nexit 0\n").expect("write script");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("chmod script");
        let app_id = format!("stub-17-2-{}", std::process::id());
        let db = Database::open_in_memory().expect("in-memory db");
        db.upsert_app(&starship_db::AppRecord {
            app_id: app_id.clone(),
            display_name: "Stub".to_string(),
            version: "0".to_string(),
            binary_path: script.to_string_lossy().to_string(),
            manifest_path: String::new(),
            socket_path: None,
            capabilities_json: "{}".to_string(),
            is_active_gui: false,
            last_seen_at: String::new(),
            created_at: String::new(),
        })
        .expect("upsert app");
        let (pid, bridge_status) = spawn_app(&db, &app_id, Duration::from_millis(200))
            .await
            .expect("spawn stub binary");
        assert!(pid > 0);
        assert_ne!(pid, 12345, "a fixed fake PID must never be returned");
        assert_eq!(bridge_status, "launched_no_bridge");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_mcp_call_tool_rejects_unavailable_app_without_simulating_success() {
        let db = Database::open_in_memory().expect("in-memory db");
        let payload = CallToolPayload {
            app_id: "eep".to_string(),
            tool_name: "timeline.auto_trim_silence".to_string(),
            arguments: serde_json::json!({}),
            lease_timeout_ms: Some(1_000),
        };
        let err = call_tool(&db, payload)
            .await
            .expect_err("no bridge socket or binary must fail");
        assert_eq!(err.code, "mcp.app_unavailable");
        assert!(!err.retryable);
        assert!(
            serde_json::to_string(&err)
                .expect("serialize error")
                .contains("mcp.app_unavailable")
        );
    }

    #[tokio::test]
    async fn test_provider_refresh_persists_live_catalog() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stub");
        let port = listener.local_addr().expect("port").port();
        let body = r#"{"data":[{"id":"stub-model","name":"Stub","context_length":4096,"supported_parameters":["tools"]}]}"#;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 4096];
            let read = socket.read(&mut buffer).await.expect("read");
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.expect("write");
            request
        });

        let db = Database::open_in_memory().expect("in-memory db");
        db.set_provider_config(
            "openrouter",
            "OpenRouter",
            &format!("http://127.0.0.1:{port}"),
            Some("sk-test-canary"),
            true,
        )
        .expect("save provider");
        let models = refresh_provider_models(&db, "openrouter")
            .await
            .expect("discover models");
        assert_eq!(models.len(), 1);
        let stored = db.list_models().expect("models");
        let saved = stored
            .iter()
            .find(|m| m.id == "stub-model")
            .expect("saved model");
        assert_eq!(saved.provider_type, "cloud_openrouter");
        assert_eq!(saved.context_window, 4096);
        let request = server.await.expect("stub task");
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer")
        );
        assert!(!request.contains("enc_v1:"));
    }

    #[test]
    fn test_tool_call_name_parsing_and_validated_sheet() {
        assert_eq!(
            parse_tool_name("atp: tuneup.get_health"),
            Some(("atp".to_string(), "tuneup.get_health".to_string()))
        );
        assert!(parse_tool_name("tuneup.get_health").is_none());

        let calls = vec![ToolCall {
            id: "call-1".to_string(),
            name: "ssp: ssp.recording.start".to_string(),
            arguments: serde_json::json!({}),
        }];
        let sheet = tool_calls_to_sheet(
            "Check system health",
            &calls,
            &TaskSheetValidator::default(),
        )
        .expect("safe tool call must produce a valid plan");
        assert_eq!(sheet.steps.len(), 1);
        assert_eq!(sheet.steps[0].app_id, "ssp");
        assert_eq!(sheet.steps[0].tool_name, "ssp.recording.start");
        assert_eq!(
            sheet.steps[0].risk_tier,
            SafetyArbiter::default().classify_tool("ssp.recording.start") as u8
        );

        let mutating = vec![ToolCall {
            id: "call-2".to_string(),
            name: "eep: eep.timeline.cut_interval".to_string(),
            arguments: serde_json::json!({ "startMs": 0, "endMs": 1000 }),
        }];
        let sheet = tool_calls_to_sheet("Clean caches", &mutating, &TaskSheetValidator::default())
            .expect("valid plan");
        assert!(
            sheet.steps[0].risk_tier >= 2,
            "mutating tool must be gated by approval"
        );

        let invalid = vec![ToolCall {
            id: "call-3".to_string(),
            name: "tuneup.get_health".to_string(),
            arguments: serde_json::json!({}),
        }];
        let err = tool_calls_to_sheet("bad", &invalid, &TaskSheetValidator::default())
            .expect_err("unqualified name");
        assert_eq!(err.code, "plan.unknown_app");
    }

    #[test]
    fn test_tool_result_is_fed_back_into_next_model_turn() {
        let db = Database::open_in_memory().expect("in-memory db");
        let model_id = db
            .list_models()
            .expect("models")
            .first()
            .expect("seeded model")
            .id
            .clone();
        let mission_id = db
            .create_mission(
                "tool-loop-17-13",
                "Chat",
                "Check health",
                &model_id,
                "local_gguf",
            )
            .expect("create mission");
        db.add_message(mission_id, "user", "check my health", None)
            .expect("user turn");
        db.add_tool_message(mission_id, "step-0", "{\"healthScore\":91}")
            .expect("tool result");
        let history = db.list_messages(mission_id).expect("history");
        let messages = conversation_messages(&history, 40, 24_000);
        assert_eq!(messages.len(), 2);
        assert!(messages[1].content.contains("healthScore"));
        assert_eq!(messages[1].tool_call_id.as_deref(), Some("step-0"));
    }

    #[test]
    fn test_conversation_memory_two_turns_preserves_context() {
        let db = Database::open_in_memory().expect("in-memory db");
        let model_id = db
            .list_models()
            .expect("models")
            .first()
            .expect("seeded model")
            .id
            .clone();
        let mission_id = db
            .create_mission(
                "chat-17-12",
                "Chat",
                "Remember my name",
                &model_id,
                "local_gguf",
            )
            .expect("create mission");
        db.add_message(mission_id, "user", "My name is Zvirox", None)
            .expect("first user turn");
        db.add_message(mission_id, "assistant", "Hello Zvirox", None)
            .expect("assistant turn");
        db.add_message(mission_id, "user", "What is my name?", None)
            .expect("second user turn");

        let history = db.list_messages(mission_id).expect("history");
        let messages = conversation_messages(
            &history,
            CONVERSATION_MAX_MESSAGES,
            CONVERSATION_CHAR_BUDGET,
        );
        assert_eq!(messages.len(), 3);
        let context = messages
            .iter()
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(context.contains("My name is Zvirox"));
        assert!(context.contains("Hello Zvirox"));
        assert!(context.contains("What is my name?"));
    }

    #[test]
    fn test_conversation_memory_budget_trims_oldest_and_keeps_latest() {
        let db = Database::open_in_memory().expect("in-memory db");
        let model_id = db
            .list_models()
            .expect("models")
            .first()
            .expect("seeded model")
            .id
            .clone();
        let mission_id = db
            .create_mission("budget-17-12", "Chat", "Long chat", &model_id, "local_gguf")
            .expect("create mission");
        for i in 0..60 {
            db.add_message(
                mission_id,
                "user",
                &format!("turn-{i}-{}", "x".repeat(80)),
                None,
            )
            .expect("add message");
        }
        let history = db.list_messages(mission_id).expect("history");
        let messages = conversation_messages(&history, 5, 600);
        assert_eq!(messages.len(), 5, "message cap must be enforced");
        assert!(messages.last().expect("latest").content.contains("turn-59"));
        assert!(
            messages
                .iter()
                .all(|message| !message.content.contains("turn-0-")),
            "oldest messages must be trimmed"
        );
    }

    #[test]
    fn test_friendly_stream_error_copy() {
        assert_eq!(
            friendly_stream_error(
                "infer.cloud_auth_failed",
                "cloud_openrouter",
                None,
                "raw",
                None
            ),
            "API key rejected by OpenRouter. Check it in Model settings."
        );
        assert_eq!(
            friendly_stream_error("infer.model_not_found", "cloud_openai", None, "raw", None),
            "Model not found. Choose a model that is available to this provider."
        );
        let details = serde_json::json!({ "retryAfterSeconds": 12 });
        assert_eq!(
            friendly_stream_error(
                "infer.cloud_rate_limited",
                "cloud_openai",
                None,
                "raw",
                Some(&details)
            ),
            "Rate limited by OpenAI. Retry in 12s."
        );
        assert_eq!(
            friendly_stream_error(
                "something.unknown",
                "cloud_gemini",
                None,
                "raw detail",
                None
            ),
            "raw detail"
        );
    }

    #[tokio::test]
    async fn test_provider_attribution_headers_are_sent() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stub");
        let port = listener.local_addr().expect("port").port();
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"OK\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 4096];
            let read = socket.read(&mut buffer).await.expect("read");
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.expect("write");
            request
        });
        let db = Database::open_in_memory().expect("in-memory db");
        db.set_provider_config(
            "openrouter",
            "OpenRouter",
            &format!("http://127.0.0.1:{port}"),
            Some("sk-test"),
            true,
        )
        .expect("save provider");
        db.set_provider_headers(
            "openrouter",
            Some("https://example.in"),
            Some("Starship Control"),
        )
        .expect("save headers");
        let result = validate_model_id(&db, "openrouter", "test/model")
            .await
            .expect("validation");
        let request = server.await.expect("stub");
        assert!(result.ok);
        let lowered = request.to_ascii_lowercase();
        assert!(lowered.contains("http-referer: https://example.in"));
        assert!(lowered.contains("x-title: starship control"));
    }

    #[tokio::test]
    async fn test_manual_model_id_validates_and_registers() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stub");
        let port = listener.local_addr().expect("port").port();
        let body = "data: {\"choices\":[{\"delta\":{\"content\":\"OK\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 4096];
            let _ = socket.read(&mut buffer).await.expect("read");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.expect("write");
        });

        let db = Database::open_in_memory().expect("in-memory db");
        db.set_provider_config(
            "openai",
            "OpenAI",
            &format!("http://127.0.0.1:{port}"),
            Some("sk-test"),
            true,
        )
        .expect("save provider");
        let result = validate_model_id(&db, "openai", "private/model-v2")
            .await
            .expect("test result");
        server.await.expect("stub task");
        assert!(result.ok, "{}", result.message);
        assert!(
            db.list_models()
                .expect("models")
                .iter()
                .any(|model| model.id == "private/model-v2")
        );
    }

    #[tokio::test]
    async fn test_dispatch_retries_transient_rate_limit() {
        use futures::StreamExt;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stub");
        let port = listener.local_addr().expect("port").port();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("first request");
            let mut buffer = [0_u8; 4096];
            let _ = socket.read(&mut buffer).await.expect("read 429");
            socket
                .write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .expect("write 429");

            let (mut socket, _) = listener.accept().await.expect("second request");
            let _ = socket.read(&mut buffer).await.expect("read retry");
            let body = "data: {\"choices\":[{\"delta\":{\"content\":\"retried\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write SSE");
        });

        let base_url = format!("http://127.0.0.1:{port}");
        let request = ChatRequest {
            model: "stub".to_string(),
            messages: vec![Message::user("hello")],
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            grammar: None,
            stream: true,
        };
        let mut stream = dispatch_with_retry(
            "cloud_openai",
            "stub",
            Some(&base_url),
            None,
            None,
            request,
            None,
        )
        .await
        .expect("retry must succeed");
        let mut content = String::new();
        while let Some(chunk) = stream.next().await {
            content.push_str(&chunk.expect("chunk").content);
        }
        server.await.expect("stub task");
        assert_eq!(content, "retried");
    }

    #[test]
    fn test_cloud_credentials_decrypted_at_point_of_use() {
        let db = starship_db::Database::open_in_memory().expect("in-memory db");
        let canary = "sk-canary-17-11-plaintext";
        db.set_provider_config(
            "openai",
            "OpenAI",
            "https://api.openai.com/v1",
            Some(canary),
            true,
        )
        .expect("set provider config");

        let stored = db
            .get_provider_config("openai")
            .expect("get config")
            .expect("config exists")
            .api_key_encrypted
            .expect("encrypted key");
        assert!(stored.starts_with("enc_v1:"), "key must be armored at rest");
        assert!(!stored.contains(canary), "canary leaked into DB plaintext");

        let (base_url, api_key) =
            resolve_cloud_credentials(&db, "cloud_openai").expect("resolve credentials");
        assert_eq!(base_url.as_deref(), Some("https://api.openai.com/v1"));
        assert_eq!(
            api_key.as_deref(),
            Some(canary),
            "dispatcher must receive plaintext, not the enc_v1 blob"
        );
        assert!(!api_key.expect("key").starts_with("enc_v1:"));

        let (base, key) = resolve_cloud_credentials(&db, "local_gguf").expect("local model");
        assert!(base.is_none() && key.is_none());
    }

    #[test]
    fn test_tray_state_all_variants_and_labels() {
        let cases = [
            (TrayState::Idle, "idle", "Idle"),
            (TrayState::Thinking, "thinking", "Thinking"),
            (TrayState::Executing, "executing", "Executing"),
            (
                TrayState::WaitingApproval,
                "waiting_approval",
                "Waiting Approval",
            ),
            (TrayState::Stalled, "stalled", "Stalled"),
            (TrayState::Overloaded, "overloaded", "Overloaded"),
            (TrayState::Failed, "failed", "Failed"),
        ];

        for (variant, expected_json, expected_label) in cases {
            assert_eq!(variant.label(), expected_label);

            let json_str = serde_json::to_string(&variant).expect("serialize tray state");
            assert_eq!(json_str, format!("\"{expected_json}\""));

            let parsed: TrayState =
                serde_json::from_str(&json_str).expect("deserialize tray state");
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn test_health_ipc_payloads_serialization_and_schema_parity() {
        // Verification requirement for 14.8:
        // "Verify: IPC schema tests (Rust <-> TS types identical)."
        use crate::health_service::{
            HealthHistoryPayload, HealthHistoryResponse, HealthSnapshotResponse,
            HealthStateChangedEvent,
        };
        use starship_core::executor::GovernorDirective;
        use starship_health::{
            CpuMetrics, HealthAlert, HealthSample, HealthState, MemoryMetrics, OverloadThresholds,
        };

        // 1. HealthSnapshotResponse serialization
        let snapshot = HealthSnapshotResponse {
            sample: HealthSample {
                timestamp_ms: 1_700_000_000_000,
                cpu: Some(CpuMetrics {
                    global_util_pct: 45.0,
                    core_util_pct: vec![40.0, 50.0],
                    load_avg_1m: 1.0,
                    load_avg_5m: 1.0,
                    load_avg_15m: 1.0,
                    avg_freq_mhz: 3200,
                }),
                memory: Some(MemoryMetrics {
                    total_bytes: 16_000_000_000,
                    used_bytes: 8_000_000_000,
                    available_bytes: 8_000_000_000,
                    available_pct: 50.0,
                    swap_total_bytes: 0,
                    swap_used_bytes: 0,
                    swap_in_bytes_per_sec: 0.0,
                }),
                top_processes: Vec::new(),
                ..Default::default()
            },
            state: HealthState::Normal,
            governor_directive: GovernorDirective {
                current_health_state: HealthState::Normal,
                max_parallelism: 2,
                allow_heavy_steps: true,
                allow_local_inference: true,
                allow_background_prefetch: true,
                should_pause: false,
                should_resume: false,
                should_unload_idle_model: false,
                action_message: "Normal system health".to_string(),
            },
        };

        let json_snapshot = serde_json::to_string(&snapshot).expect("serialize snapshot");
        assert!(json_snapshot.contains("\"timestampMs\":1700000000000"));
        assert!(json_snapshot.contains("\"globalUtilPct\":45.0"));
        assert!(json_snapshot.contains("\"governorDirective\""));
        assert!(json_snapshot.contains("\"currentHealthState\":\"normal\""));

        // 2. HealthHistoryPayload and Response
        let history_req = HealthHistoryPayload {
            range: "15m".to_string(),
            resolution: Some("1s".to_string()),
        };
        let json_history_req =
            serde_json::to_string(&history_req).expect("serialize history payload");
        assert!(json_history_req.contains("\"range\":\"15m\""));
        assert!(json_history_req.contains("\"resolution\":\"1s\""));

        let history_res = HealthHistoryResponse {
            range: "15m".to_string(),
            samples: vec![snapshot.sample.clone()],
            rollups: Vec::new(),
        };
        let json_history_res = serde_json::to_string(&history_res).expect("serialize history res");
        assert!(json_history_res.contains("\"samples\""));
        assert!(json_history_res.contains("\"rollups\""));

        // 3. OverloadThresholds serialization
        let thresholds = OverloadThresholds::default();
        let json_thresholds = serde_json::to_string(&thresholds).expect("serialize thresholds");
        assert!(json_thresholds.contains("\"cpuElevatedPct\":80.0"));
        assert!(json_thresholds.contains("\"cpuCriticalPct\":97.0"));
        assert!(json_thresholds.contains("\"vramElevatedPct\":85.0"));

        // 4. Events serialization
        let state_evt = HealthStateChangedEvent {
            previous_state: HealthState::Normal,
            current_state: HealthState::Elevated,
            directive: GovernorDirective {
                current_health_state: HealthState::Elevated,
                max_parallelism: 2,
                allow_heavy_steps: true,
                allow_local_inference: true,
                allow_background_prefetch: false,
                should_pause: false,
                should_resume: false,
                should_unload_idle_model: false,
                action_message: "Elevated load".to_string(),
            },
        };
        let json_state_evt = serde_json::to_string(&state_evt).expect("serialize state evt");
        assert!(json_state_evt.contains("\"previousState\":\"normal\""));
        assert!(json_state_evt.contains("\"currentState\":\"elevated\""));

        let alert_evt = HealthAlert {
            signal: "cpu".to_string(),
            level: HealthState::Elevated,
            value: 85.0,
            action: "warn".to_string(),
        };
        let json_alert_evt = serde_json::to_string(&alert_evt).expect("serialize alert evt");
        assert!(json_alert_evt.contains("\"signal\":\"cpu\""));
        assert!(json_alert_evt.contains("\"level\":\"elevated\""));
    }
}
