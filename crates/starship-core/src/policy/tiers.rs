use crate::safety::RiskTier;
use serde::Deserialize;
use std::collections::HashMap;

const DEFAULT_TIERS_TOML: &str = include_str!("tiers.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct TierPolicyConfig {
    pub defaults: DefaultsConfig,
    pub patterns: PatternsConfig,
    #[serde(default)]
    pub allowlist: AllowlistConfig,
    #[serde(default)]
    pub explicit_floors: HashMap<String, u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsConfig {
    pub unknown_tool_floor: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PatternsConfig {
    pub destructive_keywords: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AllowlistConfig {
    #[serde(default)]
    pub safe_overrides: Vec<String>,
}

/// Host-enforced safety tier policy manager.
/// Enforces: Effective tier = `max(app_declared, host_floor)`.
/// Unknown tools default to Tier 2. Destructive keywords force Tier 3 unless allowlisted.
/// Sub-applications can raise the risk tier, but never lower it.
#[derive(Debug, Clone)]
pub struct TierPolicy {
    config: TierPolicyConfig,
}

impl Default for TierPolicy {
    fn default() -> Self {
        Self::from_toml(DEFAULT_TIERS_TOML).expect("embedded tiers.toml is valid")
    }
}

impl TierPolicy {
    /// Loads a tier policy from TOML configuration string.
    ///
    /// # Errors
    /// Returns `toml::de::Error` if TOML fails deserialization.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        let config: TierPolicyConfig = toml::from_str(toml_str)?;
        Ok(Self { config })
    }

    /// Evaluates the host-enforced floor for a given tool name.
    #[must_use]
    pub fn host_floor(&self, tool_name: &str) -> RiskTier {
        let lower = tool_name.to_ascii_lowercase();

        // 1. Allowlist overrides take precedence (e.g. read-only tools matching keywords)
        if self
            .config
            .allowlist
            .safe_overrides
            .iter()
            .any(|item| item.eq_ignore_ascii_case(tool_name))
        {
            return RiskTier::Safe;
        }

        // 2. Explicit tool mappings from policy configuration
        if let Some(tier) = self
            .config
            .explicit_floors
            .get(tool_name)
            .copied()
            .and_then(RiskTier::from_u8)
        {
            return tier;
        }

        // 3. Keyword matching for destructive operations (Floor: Tier 3)
        if self
            .config
            .patterns
            .destructive_keywords
            .iter()
            .any(|kw| lower.contains(kw))
        {
            return RiskTier::Destructive;
        }

        // 4. Recognized read-only tool prefixes/keywords
        if lower.starts_with("get_")
            || lower.starts_with("list_")
            || lower.starts_with("read_")
            || lower.starts_with("inspect_")
            || lower.starts_with("scan_")
            || lower.starts_with("status_")
            || lower.starts_with("query_")
            || lower.contains(".get")
            || lower.contains(".list")
            || lower.contains(".read")
            || lower.contains(".inspect")
            || lower.contains(".scan")
            || lower.contains(".status")
            || lower.contains(".query")
            || lower.contains(".find")
            || lower.contains(".ping")
        {
            return RiskTier::Safe;
        }

        // 5. Default floor for unknown tools (Floor: Tier 2)
        RiskTier::from_u8(self.config.defaults.unknown_tool_floor).unwrap_or(RiskTier::Mutating)
    }

    /// Calculates the effective risk tier.
    /// Invariant: `effective = max(app_declared, host_floor)`.
    /// Apps can raise the tier (e.g., mark a read-only tool as mutating), but never lower it.
    #[must_use]
    pub fn calculate_effective_tier(
        &self,
        app_declared: Option<RiskTier>,
        tool_name: &str,
    ) -> RiskTier {
        let floor = self.host_floor(tool_name);
        match app_declared {
            Some(declared) => std::cmp::max(declared, floor),
            None => floor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase<'a> {
        name: &'a str,
        tool_name: &'a str,
        app_declared: Option<RiskTier>,
        expected: RiskTier,
    }

    #[test]
    fn test_table_driven_tier_policy() {
        let policy = TierPolicy::default();

        let cases = vec![
            // Host floor elevation: app declaring Tier 1 for delete must be elevated to Tier 3
            TestCase {
                name: "malicious/buggy app claims file delete is Tier 1 Safe",
                tool_name: "files.delete",
                app_declared: Some(RiskTier::Safe),
                expected: RiskTier::Destructive, // Tier 3
            },
            TestCase {
                name: "app claims purge cache is Tier 1 Safe",
                tool_name: "atp.purge_package_cache",
                app_declared: Some(RiskTier::Safe),
                expected: RiskTier::Destructive, // Tier 3
            },
            TestCase {
                name: "tool with remove keyword",
                tool_name: "timeline.remove_track",
                app_declared: Some(RiskTier::Safe),
                expected: RiskTier::Destructive,
            },
            TestCase {
                name: "tool with overwrite keyword",
                tool_name: "media.overwrite_stream",
                app_declared: Some(RiskTier::Mutating),
                expected: RiskTier::Destructive,
            },
            TestCase {
                name: "tool with truncate keyword",
                tool_name: "database.truncate_table",
                app_declared: None,
                expected: RiskTier::Destructive,
            },
            // Unknown tool defaults to Tier 2
            TestCase {
                name: "completely unknown custom tool with no declaration",
                tool_name: "random_vendor_tool",
                app_declared: None,
                expected: RiskTier::Mutating, // Tier 2
            },
            TestCase {
                name: "unknown tool claiming Safe is elevated to Mutating floor",
                tool_name: "vendor.do_action",
                app_declared: Some(RiskTier::Safe),
                expected: RiskTier::Mutating, // Tier 2
            },
            // Allowlisted safe overrides
            TestCase {
                name: "allowlisted safe override matching delete keyword",
                tool_name: "trash.list_deleted",
                app_declared: Some(RiskTier::Safe),
                expected: RiskTier::Safe,
            },
            // App raising the tier is honored
            TestCase {
                name: "read-only tool raised to Tier 2 by app",
                tool_name: "media.inspect_frames",
                app_declared: Some(RiskTier::Mutating),
                expected: RiskTier::Mutating,
            },
            TestCase {
                name: "read-only tool raised to Tier 3 by app",
                tool_name: "system.query_status",
                app_declared: Some(RiskTier::Destructive),
                expected: RiskTier::Destructive,
            },
            // Normal read-only tool
            TestCase {
                name: "standard inspection query",
                tool_name: "nwp.inspect_sockets",
                app_declared: Some(RiskTier::Safe),
                expected: RiskTier::Safe,
            },
        ];

        for case in cases {
            let actual = policy.calculate_effective_tier(case.app_declared, case.tool_name);
            assert_eq!(
                actual, case.expected,
                "FAILED case '{}': tool '{}', declared: {:?}, expected {:?}, got {:?}",
                case.name, case.tool_name, case.app_declared, case.expected, actual
            );
        }
    }
}
