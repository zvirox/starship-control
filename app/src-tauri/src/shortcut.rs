use ashpd::desktop::CreateSessionOptions;
use ashpd::desktop::global_shortcuts::{BindShortcutsOptions, GlobalShortcuts, NewShortcut};
use futures::StreamExt;
use tauri::AppHandle;
use tracing::{info, warn};

/// Initializes the Wayland `GlobalShortcuts` portal for Starship Control (`Super + Shift + Space`).
/// Handles portal permission dialog gracefully with educational first-run notice (`PLAN.md §6.2`).
pub fn register_wayland_global_shortcut(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        match setup_portal_shortcut(app).await {
            Ok(()) => {
                info!(target: "starship_portal", "Wayland GlobalShortcuts portal loop completed");
            }
            Err(e) => {
                warn!(
                    target: "starship_portal",
                    "Wayland GlobalShortcuts portal registration skipped or failed: {e}. Fallback to in-app shortcut."
                );
            }
        }
    });
}

async fn setup_portal_shortcut(app: AppHandle) -> ashpd::Result<()> {
    let proxy = GlobalShortcuts::new().await?;
    let session = proxy
        .create_session(CreateSessionOptions::default())
        .await?;

    info!(
        target: "starship_portal",
        "Binding global shortcut 'Super+Shift+Space' for HUD toggle via XDG Desktop Portal"
    );

    let request = proxy
        .bind_shortcuts(
            &session,
            &[
                NewShortcut::new("toggle_hud", "Toggle Starship Control Floating HUD")
                    .preferred_trigger("Super+Shift+Space"),
            ],
            None,
            BindShortcutsOptions::default(),
        )
        .await;

    match request {
        Ok(response) => {
            let shortcuts = response.response()?;
            info!(target: "starship_portal", "Registered shortcuts via portal: {:?}", shortcuts.shortcuts());

            let mut stream = proxy.receive_activated().await?;
            while let Some(activated) = stream.next().await {
                if activated.shortcut_id() == "toggle_hud" {
                    info!(target: "starship_portal", "Global portal shortcut triggered: toggle_hud");
                    let _ = crate::ipc::cmd_toggle_hud(app.clone()).await;
                }
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}
