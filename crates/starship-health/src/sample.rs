use serde::{Deserialize, Serialize};

/// High-resolution system health sample collected at up to 1 Hz (GAP-CLOSURE-PLAN.md Phase 14).
/// All sensor groups are `Option<T>` to provide graceful degradation when sensors are unavailable.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthSample {
    pub timestamp_ms: u64,
    pub cpu: Option<CpuMetrics>,
    pub psi: Option<PsiMetrics>,
    pub memory: Option<MemoryMetrics>,
    pub disk: Option<DiskMetrics>,
    pub network: Option<NetworkMetrics>,
    pub thermal: Option<ThermalMetrics>,
    pub gpu: Option<GpuMetrics>,
    pub power: Option<PowerMetrics>,
    pub top_processes: Vec<ProcessMetrics>,
    pub own_footprint: Option<FootprintMetrics>,
}

/// CPU utilization metrics across all cores and load average.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CpuMetrics {
    pub global_util_pct: f32,
    pub core_util_pct: Vec<f32>,
    pub load_avg_1m: f64,
    pub load_avg_5m: f64,
    pub load_avg_15m: f64,
    pub avg_freq_mhz: u64,
}

/// Linux Pressure Stall Information (PSI) for CPU, Memory, and I/O.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PsiMetrics {
    pub cpu_some_avg10: f64,
    pub cpu_some_avg60: f64,
    pub mem_some_avg10: f64,
    pub mem_some_avg60: f64,
    pub mem_full_avg10: f64,
    pub mem_full_avg60: f64,
    pub io_some_avg10: f64,
    pub io_some_avg60: f64,
    pub io_full_avg10: f64,
    pub io_full_avg60: f64,
}

/// System memory metrics with specific focus on `MemAvailable` and swap-in rates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetrics {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub available_pct: f32,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_in_bytes_per_sec: f64,
}

/// Disk utilization and throughput metrics per monitored mount.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiskMetrics {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub free_pct: f32,
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,
}

/// Network traffic bandwidth metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkMetrics {
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
    pub connection_count: u32,
}

/// Hardware thermal temperatures and fan speeds from sysfs hwmon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThermalMetrics {
    pub max_temp_celsius: f32,
    pub crit_temp_celsius: Option<f32>,
    pub delta_to_max: f32,
    pub fan_rpm: Option<u32>,
}

/// GPU compute and memory utilization (NVML/AMDGPU).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpuMetrics {
    pub device_name: String,
    pub util_pct: f32,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
    pub vram_used_pct: f32,
    pub temp_celsius: Option<f32>,
}

/// Power supply and battery metrics (laptop guard).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PowerMetrics {
    pub on_ac_power: bool,
    pub battery_pct: Option<f32>,
}

/// Top process resource consumers tagged by Starship ownership.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMetrics {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub mem_bytes: u64,
    pub is_starship_owned: bool,
}

/// Starship Control's own combined footprint (core + child sub-apps).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FootprintMetrics {
    pub total_cpu_pct: f32,
    pub total_rss_bytes: u64,
    pub open_sockets: u32,
    pub process_count: u32,
}
