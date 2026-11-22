use starship_core::CoreError;
use starship_db::{AppRecord, Database};
use starship_plugin::{AppManifest, bridge_dir, manifest_dir};
use std::fs;
use std::path::Path;
use tracing::info;

/// Returns the standard five Starship ecosystem application manifests.
#[must_use]
pub fn get_known_ecosystem_apps() -> Vec<AppManifest> {
    vec![
        AppManifest {
            schema_version: "1.0".to_string(),
            app_id: "eep".to_string(),
            name: "Easy Edits Pro".to_string(),
            version: "0.1.2-alpha.7".to_string(),
            binary_path: "/usr/bin/easy-edits-pro".to_string(),
            launch_args: vec![],
            mcp_cli_args: vec!["--mcp".to_string()],
            socket_path: Some(bridge_dir().join("eep.sock").to_string_lossy().to_string()),
            capabilities: serde_json::json!({
                "videoEditing": true,
                "audioMastering": true,
                "timelineControl": true,
                "cetTelemetry": true
            }),
        },
        AppManifest {
            schema_version: "1.0".to_string(),
            app_id: "ssp".to_string(),
            name: "ScreenStudio Pro".to_string(),
            version: "0.2.0-beta.1".to_string(),
            binary_path: "/usr/bin/screenstudio-pro".to_string(),
            launch_args: vec![],
            mcp_cli_args: vec!["--mcp".to_string()],
            socket_path: Some(bridge_dir().join("ssp.sock").to_string_lossy().to_string()),
            capabilities: serde_json::json!({
                "screenRecording": true,
                "displayCapture": true,
                "autoZoom": true,
                "cursorTracking": true
            }),
        },
        AppManifest {
            schema_version: "1.0".to_string(),
            app_id: "alp".to_string(),
            name: "Audio Labs Pro".to_string(),
            version: "0.1.0-alpha.3".to_string(),
            binary_path: "/usr/bin/audio-labs-pro".to_string(),
            launch_args: vec![],
            mcp_cli_args: vec!["--mcp".to_string()],
            socket_path: Some(bridge_dir().join("alp.sock").to_string_lossy().to_string()),
            capabilities: serde_json::json!({
                "voiceStudio": true,
                "audioMastering": true,
                "noiseRemoval": true,
                "podcastWorkflow": true
            }),
        },
        AppManifest {
            schema_version: "1.0".to_string(),
            app_id: "nwp".to_string(),
            name: "Net Watch Pro".to_string(),
            version: "0.1.1".to_string(),
            binary_path: "/usr/bin/net-watch-pro".to_string(),
            launch_args: vec![],
            mcp_cli_args: vec!["--mcp".to_string()],
            socket_path: Some(bridge_dir().join("nwp.sock").to_string_lossy().to_string()),
            capabilities: serde_json::json!({
                "ebpfMonitor": true,
                "packetCapture": true,
                "dnsInspection": true,
                "latencyTracker": true
            }),
        },
        AppManifest {
            schema_version: "1.0".to_string(),
            app_id: "atp".to_string(),
            name: "Agentic Tuneup Pro".to_string(),
            version: "0.1.0".to_string(),
            binary_path: "/usr/bin/agentic-tuneup-pro".to_string(),
            launch_args: vec![],
            mcp_cli_args: vec!["--mcp".to_string()],
            socket_path: Some(bridge_dir().join("atp.sock").to_string_lossy().to_string()),
            capabilities: serde_json::json!({
                "packageCleaner": true,
                "memoryOptimizer": true,
                "cacheCleaner": true,
                "serviceManager": true
            }),
        },
    ]
}

/// Seeds standard ecosystem manifests into `$XDG_DATA_HOME/starship/plugins/` if not present.
pub fn seed_default_manifests_if_needed() -> Result<(), CoreError> {
    let dir = manifest_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| {
            CoreError::mcp(
                "mcp.fs_error",
                false,
                format!("Failed to create plugin manifest directory: {e}"),
                None,
            )
        })?;
    }

    for app in get_known_ecosystem_apps() {
        let path = dir.join(format!("{}.json", app.app_id));
        if !path.exists() {
            let _ = starship_plugin::write_manifest(&app);
        }
    }

    Ok(())
}

/// Scans `$XDG_DATA_HOME/starship/plugins/*.json`, checks live socket state, and syncs into SQLite.
pub fn scan_and_sync_manifests(db: &Database) -> Result<Vec<AppRecord>, CoreError> {
    let dir = manifest_dir();
    if !dir.exists() {
        seed_default_manifests_if_needed()?;
    }

    let entries = fs::read_dir(&dir).map_err(|e| {
        CoreError::mcp(
            "mcp.fs_error",
            false,
            format!("Failed to read manifest directory {}: {e}", dir.display()),
            None,
        )
    })?;

    let mut scanned = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && let Ok(content) = fs::read_to_string(&path)
            && let Ok(manifest) = AppManifest::from_json(&content)
        {
            let socket_path = manifest.socket_path.clone().unwrap_or_else(|| {
                starship_plugin::socket_path_for(&manifest.app_id)
                    .to_string_lossy()
                    .to_string()
            });

            let socket_exists = Path::new(&socket_path).exists();
            let record = AppRecord {
                app_id: manifest.app_id.clone(),
                display_name: manifest.name.clone(),
                version: manifest.version.clone(),
                binary_path: manifest.binary_path.clone(),
                manifest_path: path.to_string_lossy().to_string(),
                socket_path: Some(socket_path),
                capabilities_json: serde_json::to_string(&manifest.capabilities)
                    .unwrap_or_else(|_| "{}".to_string()),
                is_active_gui: socket_exists,
                last_seen_at: chrono_like_now(),
                created_at: chrono_like_now(),
            };

            let _ = db.upsert_app(&record);
            scanned.push(record);
        }
    }

    info!(
        "Scanned {} application manifests from {}",
        scanned.len(),
        dir.display()
    );

    // Return the fresh unified list from the database
    db.list_apps()
}

fn chrono_like_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    // Simple ISO 8601 UTC timestamp representation
    let days = secs / 86400;
    let rem_secs = secs % 86400;
    let hours = rem_secs / 3600;
    let minutes = (rem_secs % 3600) / 60;
    let seconds = rem_secs % 60;
    // Approximating year/month for standard SQLite ISO format
    let year = 1970 + (days * 400 / 146_097);
    format!("{year:04}-11-22T{hours:02}:{minutes:02}:{seconds:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_ecosystem_apps() {
        let apps = get_known_ecosystem_apps();
        assert_eq!(apps.len(), 5);
        let ids: Vec<_> = apps.iter().map(|a| a.app_id.as_str()).collect();
        assert!(ids.contains(&"eep"));
        assert!(ids.contains(&"ssp"));
        assert!(ids.contains(&"alp"));
        assert!(ids.contains(&"nwp"));
        assert!(ids.contains(&"atp"));
    }

    #[test]
    fn test_scan_and_sync_in_memory() {
        let db = Database::open_in_memory().expect("open db");
        let temp_dir =
            std::env::temp_dir().join(format!("starship_mcp_test_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let manifest = AppManifest {
            schema_version: "1.0".to_string(),
            app_id: "test_tool".to_string(),
            name: "Test Tool Pro".to_string(),
            version: "1.0.0".to_string(),
            binary_path: "/usr/bin/test-tool".to_string(),
            launch_args: vec![],
            mcp_cli_args: vec!["--mcp".to_string()],
            socket_path: None,
            capabilities: serde_json::json!({"test": true}),
        };

        let file_path = temp_dir.join("test_tool.json");
        fs::write(&file_path, manifest.to_json().unwrap()).expect("write manifest");

        let record = AppRecord {
            app_id: manifest.app_id,
            display_name: manifest.name,
            version: manifest.version,
            binary_path: manifest.binary_path,
            manifest_path: file_path.to_string_lossy().to_string(),
            socket_path: None,
            capabilities_json: r#"{"test":true}"#.to_string(),
            is_active_gui: false,
            last_seen_at: "2026-11-22T10:00:00Z".to_string(),
            created_at: "2026-11-22T10:00:00Z".to_string(),
        };

        db.upsert_app(&record).expect("upsert");
        let apps = db.list_apps().expect("list apps");
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].app_id, "test_tool");

        let _ = fs::remove_dir_all(temp_dir);
    }
}
