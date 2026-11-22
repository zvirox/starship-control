use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppManifest {
    pub app_id: String,
    pub display_name: String,
    pub version: String,
    pub binary_path: String,
    pub socket_path: Option<String>,
    pub capabilities: Vec<String>,
}

impl AppManifest {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = AppManifest {
            app_id: "eep".to_string(),
            display_name: "Easy Edits Pro".to_string(),
            version: "0.1.0".to_string(),
            binary_path: "/usr/bin/eep".to_string(),
            socket_path: Some("/run/user/1000/starship/bridges/eep.sock".to_string()),
            capabilities: vec!["timeline.cut".to_string()],
        };

        let json = manifest.to_json().expect("to json");
        let parsed = AppManifest::from_json(&json).expect("from json");
        assert_eq!(manifest, parsed);
    }
}
