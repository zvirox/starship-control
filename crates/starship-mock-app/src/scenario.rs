use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

pub const SCENARIO_ENV: &str = "STARSHIP_MOCK_SCENARIO";
pub const APP_ID_ENV: &str = "STARSHIP_MOCK_APP_ID";
pub const FAILURE_MODE_ENV: &str = "STARSHIP_MOCK_FAILURE_MODE";
pub const LATENCY_ENV: &str = "STARSHIP_MOCK_LATENCY_MS";
pub const PROGRESS_STEPS_ENV: &str = "STARSHIP_MOCK_PROGRESS_STEPS";
pub const DENY_CHECKPOINTS_ENV: &str = "STARSHIP_MOCK_DENY_CHECKPOINTS";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureMode {
    #[default]
    None,
    Crash,
    Hang,
    BadJson,
    SlowProgress,
    DenyCheckpoint,
    Oom,
}

impl FromStr for FailureMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        serde_json::from_value(Value::String(value.to_owned()))
            .map_err(|_| format!("unknown failure mode: {value}"))
    }
}

impl FailureMode {
    #[must_use]
    pub fn is_fatal(self) -> bool {
        matches!(self, Self::Crash | Self::Hang | Self::Oom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ToolScenario {
    pub name: String,
    pub description: String,
    pub risk_tier: u8,
    pub failure_mode: FailureMode,
    pub latency_ms: u64,
    pub progress_steps: Option<u32>,
    pub result: Value,
}

impl Default for ToolScenario {
    fn default() -> Self {
        Self {
            name: "mock.inspect".to_owned(),
            description: "Returns a deterministic inspection result".to_owned(),
            risk_tier: 1,
            failure_mode: FailureMode::None,
            latency_ms: 0,
            progress_steps: None,
            result: serde_json::json!({ "status": "ok" }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Scenario {
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub deny_checkpoints: bool,
    pub progress_steps: u32,
    pub progress_interval_ms: u64,
    pub oom_bytes: usize,
    pub tools: Vec<ToolScenario>,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            app_id: "mock".to_owned(),
            name: "Starship Mock App".to_owned(),
            version: "0.1.0".to_owned(),
            deny_checkpoints: false,
            progress_steps: 2,
            progress_interval_ms: 0,
            oom_bytes: 64 * 1024 * 1024,
            tools: vec![ToolScenario::default()],
        }
    }
}

impl Scenario {
    /// Loads a scenario from a JSON file.
    ///
    /// # Errors
    /// Returns a message when the file cannot be read or parsed.
    pub fn from_file(path: &str) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read scenario {path}: {error}"))?;
        serde_json::from_str(&contents)
            .map_err(|error| format!("cannot parse scenario {path}: {error}"))
    }

    /// Builds the scenario from environment overrides, optionally loading a scenario file first.
    ///
    /// # Errors
    /// Returns a message when the named scenario file or an override value is invalid.
    pub fn from_env() -> Result<Self, String> {
        let mut scenario = match std::env::var(SCENARIO_ENV) {
            Ok(path) => Self::from_file(&path)?,
            Err(_) => Self::default(),
        };

        if let Ok(app_id) = std::env::var(APP_ID_ENV) {
            scenario.app_id = app_id;
        }
        if let Ok(mode) = std::env::var(FAILURE_MODE_ENV) {
            let mode = FailureMode::from_str(&mode)?;
            for tool in &mut scenario.tools {
                tool.failure_mode = mode;
            }
        }
        if let Ok(latency) = std::env::var(LATENCY_ENV) {
            let latency = latency
                .parse::<u64>()
                .map_err(|e| format!("{LATENCY_ENV}: {e}"))?;
            for tool in &mut scenario.tools {
                tool.latency_ms = latency;
            }
        }
        if let Ok(steps) = std::env::var(PROGRESS_STEPS_ENV) {
            scenario.progress_steps = steps
                .parse::<u32>()
                .map_err(|e| format!("{PROGRESS_STEPS_ENV}: {e}"))?;
        }
        if let Ok(deny) = std::env::var(DENY_CHECKPOINTS_ENV) {
            scenario.deny_checkpoints =
                matches!(deny.to_lowercase().as_str(), "1" | "true" | "yes");
        }

        Ok(scenario)
    }

    #[must_use]
    pub fn tool(&self, name: &str) -> Option<&ToolScenario> {
        self.tools.iter().find(|tool| tool.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_failure_mode_parses_from_snake_case() {
        let modes = [
            "none",
            "crash",
            "hang",
            "bad_json",
            "slow_progress",
            "deny_checkpoint",
            "oom",
        ];
        for mode in modes {
            let parsed = FailureMode::from_str(mode).unwrap_or_else(|e| panic!("{mode}: {e}"));
            assert_eq!(
                serde_json::to_value(parsed).unwrap(),
                Value::String(mode.to_owned())
            );
        }
        assert!(FailureMode::from_str("unknown").is_err());
    }

    #[test]
    fn fatal_modes_are_identified() {
        assert!(FailureMode::Crash.is_fatal());
        assert!(FailureMode::Hang.is_fatal());
        assert!(FailureMode::Oom.is_fatal());
        assert!(!FailureMode::BadJson.is_fatal());
        assert!(!FailureMode::SlowProgress.is_fatal());
        assert!(!FailureMode::DenyCheckpoint.is_fatal());
    }

    #[test]
    fn scenario_defaults_are_stable() {
        let scenario = Scenario::default();
        assert_eq!(scenario.app_id, "mock");
        assert_eq!(scenario.tools.len(), 1);
        assert_eq!(scenario.tool("mock.inspect").unwrap().risk_tier, 1);
    }
}
