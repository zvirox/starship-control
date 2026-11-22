use serde::Deserialize;
use starship_plugin::ResourceProfile;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const DEFAULT_RESOURCES_TOML: &str = include_str!("resources.toml");

/// Deserialized schema for `resources.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct ResourcePolicyConfig {
    pub defaults: ResourceProfile,
    #[serde(default)]
    pub overrides: HashMap<String, ResourceProfile>,
}

/// Host-enforced resource policy manager (GAP-CLOSURE-PLAN.md §15.1).
/// Resolves effective tool computational footprint:
/// 1. Host override from `resources.toml` (highest priority)
/// 2. Sub-app declared `ResourceProfile` via SDK
/// 3. Default high-resource fallback for unprofiled/unknown tools
#[derive(Debug, Clone)]
pub struct ResourcePolicy {
    config: ResourcePolicyConfig,
}

impl Default for ResourcePolicy {
    fn default() -> Self {
        Self::from_toml(DEFAULT_RESOURCES_TOML).expect("embedded resources.toml is valid")
    }
}

impl ResourcePolicy {
    /// Loads resource policy from a TOML configuration string.
    ///
    /// # Errors
    /// Returns `toml::de::Error` if TOML fails deserialization.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        let config: ResourcePolicyConfig = toml::from_str(toml_str)?;
        Ok(Self { config })
    }

    /// Loads resource policy from a filesystem path, or returns default if not found.
    #[must_use]
    pub fn load_or_default(path: Option<&Path>) -> Self {
        if let Some(p) = path
            && let Ok(content) = fs::read_to_string(p)
            && let Ok(policy) = Self::from_toml(&content)
        {
            return policy;
        }
        Self::default()
    }

    /// Resolves the effective resource profile for a tool.
    #[must_use]
    pub fn resolve_profile(
        &self,
        tool_name: &str,
        declared: Option<&ResourceProfile>,
    ) -> ResourceProfile {
        // 1. Host override takes highest precedence
        if let Some(host_override) = self.config.overrides.get(tool_name) {
            return host_override.clone();
        }

        // 2. Sub-app declared profile
        if let Some(decl) = declared {
            return decl.clone();
        }

        // 3. Fallback to host default (unknown tools default to high)
        self.config.defaults.clone()
    }

    /// Checks if a tool has an explicit host override.
    #[must_use]
    pub fn has_override(&self, tool_name: &str) -> bool {
        self.config.overrides.contains_key(tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starship_plugin::{CpuLevel, DiskIoLevel};

    #[test]
    fn test_unknown_tool_defaults_to_high() {
        let policy = ResourcePolicy::default();
        let unknown_tool = "unknown_vendor.alien_compute";

        // Without declared profile, unknown tool defaults to High
        let profile = policy.resolve_profile(unknown_tool, None);
        assert_eq!(profile.cpu, CpuLevel::High);
        assert_eq!(profile.disk_io, DiskIoLevel::High);
        assert!(profile.long_running);
        assert!(profile.mem_mb >= 2048);
    }

    #[test]
    fn test_declared_profile_used_when_no_override() {
        let policy = ResourcePolicy::default();
        let unlisted_tool = "my_custom_app.fast_query";

        let declared = ResourceProfile::light(128);
        let profile = policy.resolve_profile(unlisted_tool, Some(&declared));

        assert_eq!(profile.cpu, CpuLevel::Low);
        assert_eq!(profile.mem_mb, 128);
        assert_eq!(profile.disk_io, DiskIoLevel::Low);
        assert!(!profile.long_running);
    }

    #[test]
    fn test_host_override_takes_precedence() {
        let policy = ResourcePolicy::default();

        // Host overrides eep.timeline.export_mp4 to heavy (4096MB, GPU true)
        let tool_name = "eep.timeline.export_mp4";
        assert!(policy.has_override(tool_name));

        // Sub-app falsely claims export is light (100MB, no GPU)
        let false_claim = ResourceProfile::light(100);
        let effective = policy.resolve_profile(tool_name, Some(&false_claim));

        // Host override wins!
        assert_eq!(effective.cpu, CpuLevel::High);
        assert_eq!(effective.mem_mb, 4096);
        assert!(effective.gpu);
        assert_eq!(effective.disk_io, DiskIoLevel::High);
        assert!(effective.long_running);
    }

    #[test]
    fn test_custom_toml_policy() {
        let custom_toml = r#"
[defaults]
cpu = "high"
mem_mb = 8192
gpu = true
disk_io = "high"
long_running = true

[overrides."test.tool"]
cpu = "low"
mem_mb = 64
gpu = false
disk_io = "low"
long_running = false
"#;
        let policy = ResourcePolicy::from_toml(custom_toml).expect("valid custom toml");
        let profile = policy.resolve_profile("some.unknown", None);
        assert_eq!(profile.mem_mb, 8192);
        assert!(profile.gpu);

        let overridden = policy.resolve_profile("test.tool", None);
        assert_eq!(overridden.cpu, CpuLevel::Low);
        assert_eq!(overridden.mem_mb, 64);
    }
}
