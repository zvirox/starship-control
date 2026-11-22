use serde::{Deserialize, Serialize};

/// CPU resource consumption tier for an individual tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CpuLevel {
    Low,
    Med,
    High,
}

/// Disk I/O intensity tier for an individual tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiskIoLevel {
    Low,
    High,
}

/// Resource profile describing the expected computational footprint of a tool
/// (GAP-CLOSURE-PLAN.md §15.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProfile {
    pub cpu: CpuLevel,
    #[serde(alias = "mem_mb")]
    pub mem_mb: u64,
    pub gpu: bool,
    #[serde(alias = "disk_io")]
    pub disk_io: DiskIoLevel,
    #[serde(alias = "long_running")]
    pub long_running: bool,
}

impl Default for ResourceProfile {
    /// Canonical default for unknown tools: heavy resource footprint (GAP-CLOSURE-PLAN.md §15.1).
    fn default() -> Self {
        Self {
            cpu: CpuLevel::High,
            mem_mb: 2048,
            gpu: false,
            disk_io: DiskIoLevel::High,
            long_running: true,
        }
    }
}

impl ResourceProfile {
    /// Creates a high-resource profile assigned to unknown or unprofiled tools.
    #[must_use]
    pub fn unknown_default() -> Self {
        Self::default()
    }

    /// Convenience builder for lightweight, non-mutating inspection tools.
    #[must_use]
    pub fn light(mem_mb: u64) -> Self {
        Self {
            cpu: CpuLevel::Low,
            mem_mb,
            gpu: false,
            disk_io: DiskIoLevel::Low,
            long_running: false,
        }
    }

    /// Convenience builder for medium processing tools.
    #[must_use]
    pub fn medium(mem_mb: u64) -> Self {
        Self {
            cpu: CpuLevel::Med,
            mem_mb,
            gpu: false,
            disk_io: DiskIoLevel::Low,
            long_running: false,
        }
    }

    /// Convenience builder for heavy rendering, training, or export tools.
    #[must_use]
    pub fn heavy(mem_mb: u64, gpu: bool) -> Self {
        Self {
            cpu: CpuLevel::High,
            mem_mb,
            gpu,
            disk_io: DiskIoLevel::High,
            long_running: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_default_is_high() {
        let profile = ResourceProfile::unknown_default();
        assert_eq!(profile.cpu, CpuLevel::High);
        assert_eq!(profile.disk_io, DiskIoLevel::High);
        assert!(profile.long_running);
        assert!(profile.mem_mb >= 2048);
    }

    #[test]
    fn test_camel_case_json_serialization() {
        let profile = ResourceProfile {
            cpu: CpuLevel::Low,
            mem_mb: 256,
            gpu: false,
            disk_io: DiskIoLevel::Low,
            long_running: false,
        };

        let json = serde_json::to_string(&profile).expect("serialize");
        assert_eq!(
            json,
            r#"{"cpu":"low","memMb":256,"gpu":false,"diskIo":"low","longRunning":false}"#
        );

        let parsed: ResourceProfile = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, profile);
    }
}
