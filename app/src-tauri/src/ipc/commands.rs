use futures::StreamExt;
use serde::{Deserialize, Serialize};
use starship_core::{CoreError, ErrorKind};
use starship_db::Database;
use starship_db::schema::{MissionMessageRecord, ModelRecord};
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

fn chrono_now_iso() -> String {
    // Fast ISO 8601 UTC timestamp generator
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    format!("{secs}")
}
