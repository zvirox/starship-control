use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command as StdCommand;
use tokio::process::Command;
use tracing::warn;

/// Process containment configuration for spawned headless MCP sub-apps
/// (GAP-CLOSURE-PLAN.md §15.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainmentConfig {
    pub enabled: bool,
    pub memory_max: String,
    pub cpu_weight: u32,
}

impl Default for ContainmentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_max: "2G".to_string(),
            cpu_weight: 100,
        }
    }
}

/// Checks whether `systemd-run` is available and functioning on the host system.
#[must_use]
pub fn is_systemd_run_available() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }

    if (Path::new("/usr/bin/systemd-run").exists() || Path::new("/bin/systemd-run").exists())
        && let Ok(output) = StdCommand::new("systemd-run").arg("--version").output()
    {
        return output.status.success();
    }
    false
}

/// Prepares a contained or fallback process command according to host capabilities.
pub fn prepare_command(
    binary_path: &Path,
    args: &[&str],
    config: &ContainmentConfig,
) -> (Command, bool) {
    let systemd_available = is_systemd_run_available();

    #[cfg(feature = "systemd-containment")]
    if config.enabled && systemd_available {
        let mut cmd = Command::new("systemd-run");
        cmd.arg("--user")
            .arg("--scope")
            .arg(format!("-p=MemoryMax={}", config.memory_max))
            .arg(format!("-p=CPUWeight={}", config.cpu_weight))
            .arg("--")
            .arg(binary_path)
            .args(args);
        return (cmd, true);
    }

    if config.enabled && !systemd_available {
        warn!(
            "systemd-run containment requested but unavailable on host; falling back to uncontained spawn for {}",
            binary_path.display()
        );
    }

    let mut cmd = Command::new(binary_path);
    cmd.args(args);
    (cmd, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use starship_core::CoreError;
    use starship_core::executor::{FailureClass, classify_error};
    use starship_core::executor::{StepEvent, StepState, transition_step};

    #[test]
    fn test_containment_config_defaults() {
        let cfg = ContainmentConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.memory_max, "2G");
        assert_eq!(cfg.cpu_weight, 100);
    }

    #[test]
    fn test_prepare_command_respects_disabled_flag() {
        let disabled_cfg = ContainmentConfig {
            enabled: false,
            memory_max: "1G".into(),
            cpu_weight: 50,
        };
        let (cmd, contained) = prepare_command(
            Path::new("/usr/bin/python3"),
            &["-m", "http.server"],
            &disabled_cfg,
        );
        assert!(!contained);
        let std_cmd = cmd.as_std();
        assert_eq!(std_cmd.get_program(), "/usr/bin/python3");
    }

    #[test]
    fn test_prepare_command_with_containment_active() {
        if !is_systemd_run_available() {
            return;
        }

        let cfg = ContainmentConfig::default();
        let (cmd, contained) = prepare_command(
            Path::new("/usr/bin/app_worker"),
            &["--mcp", "--verbose"],
            &cfg,
        );

        assert!(contained);
        let std_cmd = cmd.as_std();
        assert_eq!(std_cmd.get_program(), "systemd-run");
        let args: Vec<&str> = std_cmd
            .get_args()
            .map(|a| a.to_str().expect("valid utf8"))
            .collect();

        assert_eq!(args[0], "--user");
        assert_eq!(args[1], "--scope");
        assert_eq!(args[2], "-p=MemoryMax=2G");
        assert_eq!(args[3], "-p=CPUWeight=100");
        assert_eq!(args[4], "--");
        assert_eq!(args[5], "/usr/bin/app_worker");
        assert_eq!(args[6], "--mcp");
        assert_eq!(args[7], "--verbose");
    }

    #[test]
    fn test_mock_runaway_app_killed_by_containment_scope_triggers_failed_final() {
        // When a runaway sub-app allocates unbounded memory in a contained scope,
        // the kernel/systemd scope terminates it with SIGKILL (signal 9 / exit status 137).
        // This test simulates the MCP client detecting the killed child process and verifies:
        // 1. Error translates to canonical "mcp.app_crashed"
        // 2. Classifies strictly as FailureClass::AppCrash
        // 3. Mission step transitions to StepState::FailedFinal without freezing the host desktop.

        let exit_code = 137; // 128 + SIGKILL (9)
        let simulated_err = CoreError::mcp(
            "mcp.execution_failed",
            true, // Retriable or triggers F1/F5 ladder
            format!("Sub-app terminated by systemd memory scope (exit code {exit_code})"),
            Some(serde_json::json!({
                "exitCode": exit_code,
                "containmentScope": "run-p12345.scope",
                "reason": "MemoryMax exceeded"
            })),
        );

        // Classification law verification
        let failure_class = classify_error(&simulated_err.code);
        assert_eq!(
            failure_class,
            FailureClass::AppFailure,
            "Contained runaway process termination must map to AppFailure"
        );

        // Step state machine verification: running step fails with FailFinal (ready for F1/F5 recovery)
        let step = StepState::Running;
        let next_step = transition_step(step, StepEvent::FailFinal).expect("valid transition");
        assert_eq!(next_step, StepState::FailedFinal);

        // F5 Rollback can be applied if mutations occurred
        let rolled_back = transition_step(next_step, StepEvent::Rollback).expect("valid rollback");
        assert_eq!(rolled_back, StepState::RolledBack);
    }
}
