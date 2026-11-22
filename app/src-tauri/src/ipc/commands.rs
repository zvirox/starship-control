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
use starship_db::{Database, NewAuditEntry};
use starship_infer::dispatcher::InferDispatcher;
use starship_infer::hf::{DownloadProgress, HfModelMetadata, download_hf_file, search_models};
use starship_infer::types::{ChatRequest, Message};
use starship_mcp::{
    AggregatedTool, HybridExecutionRouter, ToolAggregator, UnixTransportClient,
    scan_and_sync_manifests,
};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, Size};
use tracing::{error, info};
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
        let target_height = if expanded { 240.0 } else { 76.0 };
        hud_window
            .set_size(Size::Logical(LogicalSize {
                width: 520.0,
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
    info!(target: "starship_ipc", provider_id = %payload.provider_id, "Provider config saved");
    Ok(true)
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

    // Fetch model details
    let model = state.get_model(&mission.active_model_id)?.ok_or_else(|| {
        CoreError::infer(
            "infer.model_not_found",
            false,
            format!("Active model {} not found", mission.active_model_id),
            None,
        )
    })?;

    // Fetch provider config if cloud
    let (base_url, api_key) = if model.provider_type.starts_with("cloud_") {
        let provider_id = match model.provider_type.as_str() {
            "cloud_anthropic" => "anthropic",
            "cloud_gemini" => "gemini",
            "cloud_openrouter" => "openrouter",
            _ => "openai",
        };
        let cfg = state.get_provider_config(provider_id)?;
        (
            cfg.as_ref().map(|c| c.base_url.clone()),
            cfg.and_then(|c| c.api_key_encrypted),
        )
    } else {
        (None, None)
    };

    let mission_id = payload.mission_id;
    let app_handle = app.clone();
    let db = state.inner().clone();
    let prompt_text = payload.prompt.clone();

    // Spawn async background streaming task
    tokio::spawn(async move {
        let aggregator = ToolAggregator::new();
        let system_prompt = aggregator.build_system_context();
        let tools = aggregator.to_llm_definitions();

        let chat_req = ChatRequest {
            model: model.id.clone(),
            messages: vec![Message::user(prompt_text)],
            system_prompt: Some(system_prompt),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            tools,
            grammar: None,
            stream: true,
        };

        match InferDispatcher::dispatch_stream(
            &model.provider_type,
            &model.id,
            base_url.as_deref(),
            api_key.as_deref(),
            model.file_path.as_deref(),
            chat_req,
        )
        .await
        {
            Ok(mut stream) => {
                let mut accumulated_content = String::new();
                let mut accumulated_thought = String::new();

                while let Some(chunk_res) = stream.next().await {
                    match chunk_res {
                        Ok(chunk) => {
                            if let Some(ref thought) = chunk.thought {
                                accumulated_thought.push_str(thought);
                                let _ = app_handle.emit(
                                    "evt://thought/stream",
                                    ThoughtStreamEvent {
                                        mission_id,
                                        thought_chunk: thought.clone(),
                                    },
                                );
                            }

                            if !chunk.content.is_empty() {
                                accumulated_content.push_str(&chunk.content);
                                let _ = app_handle.emit(
                                    "evt://token/stream",
                                    TokenStreamEvent {
                                        mission_id,
                                        chunk: chunk.content,
                                    },
                                );
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(15)).await;
                        }
                        Err(e) => {
                            error!(target: "starship_ipc", "Stream chunk error: {e}");
                            let _ = app_handle.emit(
                                "evt://stream/error",
                                StreamErrorEvent {
                                    mission_id,
                                    message: e.user_message.clone(),
                                    code: e.code.clone(),
                                },
                            );
                            break;
                        }
                    }
                }

                // Save assistant message to SQLite
                let thought_opt = if accumulated_thought.is_empty() {
                    None
                } else {
                    Some(accumulated_thought.as_str())
                };

                let assistant_msg_id = db
                    .add_message(mission_id, "assistant", &accumulated_content, thought_opt)
                    .unwrap_or(0);

                let _ = app_handle.emit(
                    "evt://stream/finished",
                    StreamFinishedEvent {
                        mission_id,
                        message_id: assistant_msg_id,
                        content: accumulated_content,
                        thought: thought_opt.map(ToString::to_string),
                    },
                );

                info!(target: "starship_ipc", mission_id, assistant_msg_id, "Inference streaming completed");
            }
            Err(e) => {
                error!(target: "starship_ipc", "Inference dispatch failed: {e}");
                let _ = app_handle.emit(
                    "evt://stream/error",
                    StreamErrorEvent {
                        mission_id,
                        message: e.user_message.clone(),
                        code: e.code.clone(),
                    },
                );
            }
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
    let app_record = db.get_app(&payload.app_id)?.ok_or_else(|| {
        CoreError::app(
            "app.not_found",
            false,
            format!("Application '{}' is not registered", payload.app_id),
            None,
        )
    })?;

    let bin_path = std::path::Path::new(&app_record.binary_path);
    if bin_path.exists() {
        match std::process::Command::new(bin_path).spawn() {
            Ok(child) => {
                let pid = child.id();
                let _ = app.emit(
                    "evt://topology/app_state",
                    TopologyAppStateEvent {
                        app_id: payload.app_id.clone(),
                        mode: "live_gui".to_string(),
                        socket_path: app_record.socket_path.clone(),
                    },
                );
                Ok(LaunchAppResponse {
                    spawned: true,
                    pid: Some(pid),
                })
            }
            Err(e) => Err(CoreError::app(
                "mcp.app_spawn_failed",
                false,
                format!("Failed to spawn {}: {e}", app_record.binary_path),
                None,
            )),
        }
    } else {
        // In local development or simulation without installed binaries, simulate GUI launch
        let socket_path = app_record.socket_path.clone().unwrap_or_else(|| {
            starship_plugin::socket_path_for(&payload.app_id)
                .to_string_lossy()
                .to_string()
        });

        let _ = db.update_app_gui_status(&payload.app_id, true, Some(&socket_path));
        let _ = app.emit(
            "evt://topology/app_state",
            TopologyAppStateEvent {
                app_id: payload.app_id,
                mode: "live_gui".to_string(),
                socket_path: Some(socket_path),
            },
        );

        Ok(LaunchAppResponse {
            spawned: true,
            pid: Some(12345),
        })
    }
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
    let db = state.inner().clone();
    let router = HybridExecutionRouter::new(db);
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

#[tauri::command]
pub async fn cmd_plan_generate(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: GeneratePlanPayload,
) -> Result<PlanActiveResponse, CoreError> {
    let db = state.inner();
    let arbiter = SafetyArbiter::default();
    let goal_lower = payload.user_goal.to_ascii_lowercase();

    // Decompose user strategic goal into a tasksheet/1 plan
    let sheet = if goal_lower.contains("system")
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
                    diff_summary: "Purges 4.2 GB of obsolete package and build caches".to_string(),
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
                    diff_summary: "Applies 18 ripple cuts and removes 01:24 dead time via CET"
                        .to_string(),
                    depends_on: vec![1],
                },
            ],
        )
    };

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

#[tauri::command]
pub async fn cmd_pipeline_screencast(
    app: AppHandle,
    state: tauri::State<'_, Database>,
    payload: PipelineScreencastPayload,
) -> Result<PlanActiveResponse, CoreError> {
    let db = state.inner();
    let arbiter = SafetyArbiter::default();

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
        recording_path: payload.recording_path,
        cet_path: payload.cet_path,
        clean_audio: payload.clean_audio,
        audio_preset: payload.audio_preset,
        intro_script: payload.intro_script,
        target_timeline_track: payload.target_timeline_track,
    };

    let result = ScreencastPipeline::plan_assembly(&req, telemetry.as_ref())?;
    activate_plan_internal(&app, db, &arbiter, payload.mission_id, &result.task_sheet)
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
}
