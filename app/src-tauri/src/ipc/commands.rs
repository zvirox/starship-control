use serde::{Deserialize, Serialize};
use starship_core::{CoreError, ErrorKind};
use starship_db::Database;
use starship_db::schema::ModelRecord;
use tauri::{AppHandle, LogicalSize, Manager, Size};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub name: String,
    pub version: String,
    pub protocol: String,
    pub status: String,
    pub database_ready: bool,
    pub available_models: Vec<ModelRecord>,
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
