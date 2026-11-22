use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

fn default_schema_version() -> String {
    "1.0".to_string()
}

fn default_mcp_cli_args() -> Vec<String> {
    vec!["--mcp".to_string()]
}

/// Metadata describing an ecosystem application and its communication endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppManifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub app_id: String,
    #[serde(alias = "displayName")]
    pub name: String,
    pub version: String,
    pub binary_path: String,
    #[serde(default)]
    pub launch_args: Vec<String>,
    #[serde(default = "default_mcp_cli_args")]
    pub mcp_cli_args: Vec<String>,
    pub socket_path: Option<String>,
    #[serde(default)]
    pub capabilities: serde_json::Value,
}

impl AppManifest {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// A discrete tool exposed by an application plugin via MCP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginTool {
    pub name: String,
    pub description: String,
    pub risk_tier: u8,
    pub parameters: serde_json::Value,
}

/// The response payload for `starship/hello` handshake.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HelloResult {
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub tools: Vec<PluginTool>,
}

/// Returns the XDG manifest directory for starship plugins: `$XDG_DATA_HOME/starship/plugins/`.
#[must_use]
pub fn manifest_dir() -> PathBuf {
    if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(xdg_data).join("starship").join("plugins")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("starship")
            .join("plugins")
    } else {
        PathBuf::from("/tmp/starship/plugins")
    }
}

/// Returns the manifest file path for a given application ID.
#[must_use]
pub fn manifest_path_for(app_id: &str) -> PathBuf {
    manifest_dir().join(format!("{app_id}.json"))
}

/// Returns the XDG runtime directory for bridge Unix sockets: `$XDG_RUNTIME_DIR/starship/bridges/`.
#[must_use]
pub fn bridge_dir() -> PathBuf {
    if let Ok(xdg_runtime) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(xdg_runtime).join("starship").join("bridges")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("starship")
            .join("bridges")
    } else {
        PathBuf::from("/tmp/starship/bridges")
    }
}

/// Returns the socket path for a given application ID.
#[must_use]
pub fn socket_path_for(app_id: &str) -> PathBuf {
    bridge_dir().join(format!("{app_id}.sock"))
}

/// Writes the application manifest into `$XDG_DATA_HOME/starship/plugins/<app-id>.json`.
pub fn write_manifest(manifest: &AppManifest) -> io::Result<PathBuf> {
    let dir = manifest_dir();
    fs::create_dir_all(&dir)?;

    let target = manifest_path_for(&manifest.app_id);
    let json = manifest
        .to_json()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    fs::write(&target, json)?;
    Ok(target)
}

/// Removes the application manifest from `$XDG_DATA_HOME/starship/plugins/<app-id>.json`.
pub fn remove_manifest(app_id: &str) -> io::Result<()> {
    let target = manifest_path_for(app_id);
    if target.exists() {
        fs::remove_file(target)?;
    }
    Ok(())
}

/// Unix domain socket bridge server for GUI sub-applications.
pub struct BridgeServer {
    socket_path: PathBuf,
    listener: UnixListener,
}

impl BridgeServer {
    /// Binds a Unix domain socket listener at `$XDG_RUNTIME_DIR/starship/bridges/<app-id>.sock`.
    pub fn bind(app_id: &str) -> io::Result<Self> {
        let sock_path = socket_path_for(app_id);
        Self::bind_at(sock_path)
    }

    /// Binds a Unix domain socket listener at the specified file path.
    pub fn bind_at(socket_path: PathBuf) -> io::Result<Self> {
        if let Some(parent) = socket_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Clean up stale socket file if present
        if socket_path.exists() {
            let _ = fs::remove_file(&socket_path);
        }

        let listener = UnixListener::bind(&socket_path)?;
        Ok(Self {
            socket_path,
            listener,
        })
    }

    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    #[must_use]
    pub fn listener(&self) -> &UnixListener {
        &self.listener
    }
}

impl Drop for BridgeServer {
    fn drop(&mut self) {
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = AppManifest {
            schema_version: "1.0".to_string(),
            app_id: "eep".to_string(),
            name: "Easy Edits Pro".to_string(),
            version: "0.1.2-alpha.7".to_string(),
            binary_path: "/usr/bin/easy-edits-pro".to_string(),
            launch_args: vec![],
            mcp_cli_args: vec!["--mcp".to_string()],
            socket_path: Some("/run/user/1000/starship/bridges/eep.sock".to_string()),
            capabilities: serde_json::json!({
                "videoEditing": true,
                "audioMastering": true,
                "timelineControl": true
            }),
        };

        let json = manifest.to_json().expect("to json");
        let parsed = AppManifest::from_json(&json).expect("from json");
        assert_eq!(manifest, parsed);
        assert_eq!(parsed.name, "Easy Edits Pro");
    }

    #[test]
    fn test_bridge_server_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!("starship_test_{}", std::process::id()));
        let sock_path = temp_dir.join("test_app.sock");

        {
            let server = BridgeServer::bind_at(sock_path.clone()).expect("bind server");
            assert!(server.socket_path().exists());
        }

        // Drop should clean up socket file
        assert!(!sock_path.exists());
        let _ = fs::remove_dir_all(temp_dir);
    }
}
