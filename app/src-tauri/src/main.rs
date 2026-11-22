#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;
mod shortcut;

use starship_db::Database;
use tracing::info;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "starship_control=debug,starship_core=debug,starship_db=debug,starship_infer=debug,info".into()
            }),
        )
        .init();

    info!("Starting Starship Control...");

    let db = match Database::open_default() {
        Ok(db) => {
            info!(
                "SQLite database initialized successfully at {:?}",
                db.path()
            );
            db
        }
        Err(e) => {
            eprintln!("Failed to initialize database: {e}");
            panic!("Critical database initialization failure: {e}");
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            shortcut::register_wayland_global_shortcut(app.handle().clone());
            Ok(())
        })
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            ipc::ipc_hello,
            ipc::cmd_toggle_hud,
            ipc::cmd_resize_hud,
            ipc::cmd_list_models,
            ipc::cmd_mission_list,
            ipc::cmd_mission_create,
            ipc::cmd_mission_delete,
            ipc::cmd_mission_send_prompt,
            ipc::cmd_mission_list_messages,
            ipc::cmd_provider_config_set,
            ipc::cmd_model_hf_search,
            ipc::cmd_model_hf_download,
            ipc::cmd_topology_list_apps,
            ipc::cmd_topology_launch_app,
            ipc::cmd_topology_ping_app,
            ipc::cmd_mcp_list_tools,
            ipc::cmd_mcp_call_tool,
            ipc::cmd_plan_get_active,
            ipc::cmd_plan_create,
            ipc::cmd_plan_generate,
            ipc::cmd_plan_approve_step,
            ipc::cmd_plan_reject_step,
            ipc::cmd_pipeline_screencast,
            ipc::cmd_pipeline_diagnostics,
            ipc::cmd_audit_list_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
