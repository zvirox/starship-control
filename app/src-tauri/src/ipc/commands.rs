use futures::StreamExt;
use serde::{Deserialize, Serialize};
use starship_core::{CoreError, ErrorKind};
use starship_db::Database;
use starship_db::schema::{MissionMessageRecord, ModelRecord};
use starship_infer::dispatcher::InferDispatcher;
use starship_infer::types::{ChatRequest, Message};
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
pub async fn cmd_mission_list_messages(
    state: tauri::State<'_, Database>,
    mission_id: i64,
) -> Result<Vec<MissionMessageRecord>, CoreError> {
    state.list_messages(mission_id)
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
        let chat_req = ChatRequest {
            model: model.id.clone(),
            messages: vec![Message::user(prompt_text)],
            system_prompt: Some(
                "You are the Starship Control autonomous orchestrator.".to_string(),
            ),
            temperature: Some(0.2),
            max_tokens: Some(4096),
            tools: vec![],
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
                        }
                        Err(e) => {
                            error!(target: "starship_ipc", "Stream chunk error: {e}");
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
                let _ = app_handle.emit("evt://stream/error", e);
            }
        }
    });

    Ok(SendPromptResponse {
        message_id: user_msg_id,
        status: "thinking".to_string(),
    })
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
