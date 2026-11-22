use serde::{Deserialize, Serialize};
use starship_health::HealthSample;
use std::fs;

/// Specific F3 fallback path recommended when live headroom is inadequate
/// (GAP-CLOSURE-PLAN.md §15.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum F3FallbackStrategy {
    /// Reduce GPU layer offloading to fit in available VRAM, shifting weights to CPU RAM.
    OffloadFewerLayers { target_gpu_layers: u32 },
    /// Switch to a lighter quantization tier (e.g. `Q4_K_M` instead of `Q8_0`).
    SmallerQuantization { recommended_quant: String },
    /// Fall back to cloud API inference when host system memory is critically low.
    CloudFallback { recommended_provider: String },
}

/// Admission decision emitted by the local-inference guard prior to model loading.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalGuardDecision {
    /// Live headroom is adequate; proceed to load model weights.
    ReadyToLoad,
    /// Live headroom is inadequate; trigger F3 fallback before allocating to prevent OOM.
    TriggerF3Fallback {
        strategy: F3FallbackStrategy,
        user_message: String,
        details: serde_json::Value,
    },
}

/// Local-inference guard that checks live VRAM and RAM headroom prior to loading GGUF weights
/// (GAP-CLOSURE-PLAN.md §15.3).
pub struct LocalInferenceGuard;

impl LocalInferenceGuard {
    /// Safety margin in megabytes reserved for desktop OS and other applications.
    pub const RAM_SAFETY_MARGIN_MB: u64 = 768;
    pub const VRAM_SAFETY_MARGIN_MB: u64 = 256;

    /// Evaluates live memory headroom and determines whether to load or trigger F3 fallback.
    #[must_use]
    pub fn evaluate_headroom(
        model_size_mb: u64,
        gpu_layers: u32,
        health: Option<&HealthSample>,
        vulkan_supported: bool,
    ) -> LocalGuardDecision {
        let (avail_ram_mb, avail_vram_mb) = Self::resolve_live_memory(health, vulkan_supported);

        // Required memory calculation
        let required_ram_mb = model_size_mb + Self::RAM_SAFETY_MARGIN_MB;
        let required_vram_mb = if vulkan_supported && gpu_layers > 0 {
            // Pro-rated VRAM requirement based on layer offload ratio (standard 33 layers)
            let layer_ratio = (u64::from(gpu_layers)).min(33);
            ((model_size_mb * layer_ratio) / 33) + Self::VRAM_SAFETY_MARGIN_MB
        } else {
            0
        };

        // 1. Check VRAM capacity if offloading to GPU
        if vulkan_supported && gpu_layers > 0 && avail_vram_mb < required_vram_mb {
            // Check if system has sufficient CPU RAM to offload fewer layers
            if avail_ram_mb >= required_ram_mb && gpu_layers > 8 {
                let reduced_layers = (gpu_layers / 2).max(1);
                return LocalGuardDecision::TriggerF3Fallback {
                    strategy: F3FallbackStrategy::OffloadFewerLayers {
                        target_gpu_layers: reduced_layers,
                    },
                    user_message: format!(
                        "Available VRAM ({avail_vram_mb} MB) cannot fit {gpu_layers} layers. Automatically offloading {reduced_layers} layers to preserve system stability."
                    ),
                    details: serde_json::json!({
                        "avail_vram_mb": avail_vram_mb,
                        "required_vram_mb": required_vram_mb,
                        "reduced_layers": reduced_layers,
                    }),
                };
            }

            // Otherwise VRAM and/or RAM are too tight for this quantization
            return Self::recommend_smaller_quant_or_cloud(
                model_size_mb,
                avail_ram_mb,
                avail_vram_mb,
            );
        }

        // 2. Check System RAM capacity
        if avail_ram_mb < required_ram_mb {
            return Self::recommend_smaller_quant_or_cloud(
                model_size_mb,
                avail_ram_mb,
                avail_vram_mb,
            );
        }

        LocalGuardDecision::ReadyToLoad
    }

    fn recommend_smaller_quant_or_cloud(
        model_size_mb: u64,
        avail_ram_mb: u64,
        avail_vram_mb: u64,
    ) -> LocalGuardDecision {
        if avail_ram_mb >= 2500 {
            LocalGuardDecision::TriggerF3Fallback {
                strategy: F3FallbackStrategy::SmallerQuantization {
                    recommended_quant: "Q4_K_M".to_string(),
                },
                user_message: format!(
                    "Available RAM ({avail_ram_mb} MB) is tight for {model_size_mb} MB model weights. Switching to compact 4-bit quantization to prevent freezing your desktop."
                ),
                details: serde_json::json!({
                    "avail_ram_mb": avail_ram_mb,
                    "avail_vram_mb": avail_vram_mb,
                    "model_size_mb": model_size_mb,
                    "fallback_quant": "Q4_K_M"
                }),
            }
        } else {
            LocalGuardDecision::TriggerF3Fallback {
                strategy: F3FallbackStrategy::CloudFallback {
                    recommended_provider: "cloud_openrouter".to_string(),
                },
                user_message: format!(
                    "System memory is critically low ({avail_ram_mb} MB available, {model_size_mb} MB needed). Falling back to cloud inference to protect host desktop."
                ),
                details: serde_json::json!({
                    "avail_ram_mb": avail_ram_mb,
                    "avail_vram_mb": avail_vram_mb,
                    "model_size_mb": model_size_mb,
                    "fallback_provider": "cloud_openrouter"
                }),
            }
        }
    }

    fn resolve_live_memory(health: Option<&HealthSample>, vulkan_supported: bool) -> (u64, u64) {
        if let Some(h) = health {
            let ram = h.memory.as_ref().map_or_else(read_live_proc_meminfo, |m| {
                m.available_bytes / (1024 * 1024)
            });
            let vram = if vulkan_supported {
                h.gpu.as_ref().map_or(8192, |g| {
                    (g.vram_total_bytes.saturating_sub(g.vram_used_bytes)) / (1024 * 1024)
                })
            } else {
                0
            };
            (ram, vram)
        } else {
            let ram = read_live_proc_meminfo();
            let vram = if vulkan_supported { 8192 } else { 0 };
            (ram, vram)
        }
    }
}

fn read_live_proc_meminfo() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemAvailable:") {
                let kb: u64 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                if kb > 0 {
                    return kb / 1024;
                }
            }
        }
    }
    8192
}

#[cfg(test)]
mod tests {
    use super::*;
    use starship_health::MemoryMetrics;

    fn mock_health(avail_ram_mb: u64) -> HealthSample {
        HealthSample {
            memory: Some(MemoryMetrics {
                total_bytes: 16 * 1024 * 1024 * 1024,
                used_bytes: (16 * 1024 - avail_ram_mb) * 1024 * 1024,
                available_bytes: avail_ram_mb * 1024 * 1024,
                available_pct: 10.0,
                swap_total_bytes: 4 * 1024 * 1024 * 1024,
                swap_used_bytes: 0,
                swap_in_bytes_per_sec: 0.0,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn test_simulated_low_memory_triggers_fallback_with_clear_user_message_and_no_oom() {
        // Model weights are 4096 MB, but available memory is simulated at 512 MB
        let low_memory_health = mock_health(512);

        let decision = LocalInferenceGuard::evaluate_headroom(
            4096,
            0, // CPU only
            Some(&low_memory_health),
            false,
        );

        match decision {
            LocalGuardDecision::TriggerF3Fallback {
                strategy,
                user_message,
                details,
            } => {
                // Must recommend cloud fallback when available RAM is < 2500 MB
                assert_eq!(
                    strategy,
                    F3FallbackStrategy::CloudFallback {
                        recommended_provider: "cloud_openrouter".to_string()
                    }
                );
                // Clear plain-English user message
                assert!(user_message.contains("critically low"));
                assert!(user_message.contains("512 MB available"));
                assert!(user_message.contains("4096 MB needed"));
                assert_eq!(details["avail_ram_mb"], 512);
            }
            LocalGuardDecision::ReadyToLoad => {
                panic!("Expected F3 fallback under simulated low memory, got ReadyToLoad");
            }
        }
    }

    #[test]
    fn test_sufficient_memory_allows_load() {
        let good_health = mock_health(12_000);
        let decision = LocalInferenceGuard::evaluate_headroom(3000, 0, Some(&good_health), false);
        assert_eq!(decision, LocalGuardDecision::ReadyToLoad);
    }

    #[test]
    fn test_vram_constrained_triggers_layer_offload_reduction() {
        // Machine has 8000 MB RAM available, but VRAM is tight (4096 MB available vs 6256 MB required)
        let mut health = mock_health(8000);
        health.gpu = Some(starship_health::GpuMetrics {
            device_name: "AMD Radeon RX 7800 XT".to_string(),
            util_pct: 15.0,
            vram_used_bytes: 12 * 1024 * 1024 * 1024,
            vram_total_bytes: 16 * 1024 * 1024 * 1024, // 4096 MB available VRAM
            vram_used_pct: 75.0,
            temp_celsius: Some(55.0),
        });

        let decision = LocalInferenceGuard::evaluate_headroom(
            6000,
            33, // All layers to GPU
            Some(&health),
            true, // Vulkan supported, VRAM = 4096 MB < 6256 MB required
        );

        match decision {
            LocalGuardDecision::TriggerF3Fallback { strategy, .. } => {
                assert!(matches!(
                    strategy,
                    F3FallbackStrategy::OffloadFewerLayers {
                        target_gpu_layers: 16
                    }
                ));
            }
            LocalGuardDecision::ReadyToLoad => {
                panic!("Expected layer reduction fallback");
            }
        }
    }
}
