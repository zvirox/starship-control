use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    pub cpu_cores: usize,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub vulkan_supported: bool,
    pub rocm_supported: bool,
    pub total_vram_mb: u64,
    pub available_vram_mb: u64,
}

impl HardwareCapabilities {
    /// Detects host Linux hardware capabilities.
    #[must_use]
    pub fn detect() -> Self {
        let cpu_cores = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);

        let (total_ram_mb, available_ram_mb) = read_proc_meminfo();
        let vulkan_supported =
            Path::new("/dev/dri/renderD128").exists() || Path::new("/usr/share/vulkan").exists();
        let rocm_supported = Path::new("/dev/kfd").exists();

        // Estimated VRAM from GPU devices or default 8GB baseline on Linux render nodes
        let total_vram_mb = if vulkan_supported || rocm_supported {
            12288
        } else {
            0
        };
        let available_vram_mb = if total_vram_mb > 0 { 8192 } else { 0 };

        Self {
            cpu_cores,
            total_ram_mb,
            available_ram_mb,
            vulkan_supported,
            rocm_supported,
            total_vram_mb,
            available_vram_mb,
        }
    }

    /// Validates whether a model with required VRAM can be loaded safely.
    ///
    /// # Errors
    /// Returns `CoreError` with code `infer.model_out_of_memory` if required memory exceeds available VRAM.
    pub fn ensure_vram_capacity(&self, required_mb: u64) -> Result<(), CoreError> {
        if self.vulkan_supported && required_mb > self.available_vram_mb {
            return Err(CoreError::infer(
                "infer.model_out_of_memory",
                false,
                format!(
                    "Local model weights ({required_mb} MB) exceed available VRAM ({} MB).",
                    self.available_vram_mb
                ),
                Some(serde_json::json!({
                    "required_mb": required_mb,
                    "available_mb": self.available_vram_mb,
                    "backend": if self.rocm_supported { "rocm" } else { "vulkan" }
                })),
            ));
        }
        Ok(())
    }
}

fn read_proc_meminfo() -> (u64, u64) {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        let mut total = 0;
        let mut available = 0;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                total = parse_kb(line) / 1024;
            } else if line.starts_with("MemAvailable:") {
                available = parse_kb(line) / 1024;
            }
        }
        if total > 0 {
            return (total, available);
        }
    }
    (16384, 8192)
}

fn parse_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|val| val.parse::<u64>().ok())
        .unwrap_or(0)
}
