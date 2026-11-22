#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;
mod shortcut;

use starship_db::Database;
use tracing::info;

fn handle_cli_flags() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "audit" && args[2] == "verify" {
        println!("Verifying tamper-evident audit log chain...");
        match Database::open_default() {
            Ok(db) => match db.verify_audit_chain() {
                Ok(count) => {
                    println!("[OK] Audit log chain verified cleanly. {count} record(s) intact.");
                    return Some(0);
                }
                Err(e) => {
                    eprintln!("[FAIL] Audit log verification failed: {}", e.user_message);
                    return Some(1);
                }
            },
            Err(e) => {
                eprintln!("[FAIL] Database error: {e}");
                return Some(1);
            }
        }
    }

    for arg in &args[1..] {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("starship-control {}", env!("CARGO_PKG_VERSION"));
                return Some(0);
            }
            "--self-test" => {
                println!("Running Starship Control self-test...");
                match Database::open_default() {
                    Ok(db) => {
                        println!(
                            "[OK] Database opened & migrations verified at {:?}",
                            db.path()
                        );
                        println!("[OK] Self-test passed cleanly.");
                        return Some(0);
                    }
                    Err(e) => {
                        eprintln!("[FAIL] Database self-test failed: {e}");
                        return Some(1);
                    }
                }
            }
            "--verify-audit" => {
                println!("Verifying tamper-evident audit log chain...");
                match Database::open_default() {
                    Ok(db) => match db.verify_audit_chain() {
                        Ok(count) => {
                            println!(
                                "[OK] Audit log chain verified cleanly. {count} record(s) intact."
                            );
                            return Some(0);
                        }
                        Err(e) => {
                            eprintln!("[FAIL] Audit log verification failed: {}", e.user_message);
                            return Some(1);
                        }
                    },
                    Err(e) => {
                        eprintln!("[FAIL] Database error: {e}");
                        return Some(1);
                    }
                }
            }
            "--help" | "-h" => {
                println!("Starship Control — Native Linux AI Agent Orchestrator");
                println!();
                println!("Usage: starship-control [OPTIONS]");
                println!("       starship-control audit verify");
                println!();
                println!("Options:");
                println!(
                    "  --self-test     Open database, run migrations, verify health, and exit 0"
                );
                println!("  --verify-audit  Verify hash integrity of the tamper-evident audit log");
                println!("  --version, -V   Print application version and exit");
                println!("  --help, -h      Print this help message and exit");
                return Some(0);
            }
            _ => {}
        }
    }
    None
}

fn main() {
    if let Some(exit_code) = handle_cli_flags() {
        std::process::exit(exit_code);
    }
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
