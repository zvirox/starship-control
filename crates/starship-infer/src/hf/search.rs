use crate::local::hardware::HardwareCapabilities;
use serde::{Deserialize, Serialize};
use starship_core::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareFit {
    FitsInVram,
    PartialOffload,
    CpuOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HfModelMetadata {
    pub repo_id: String,
    pub filename: String,
    pub file_size_bytes: u64,
    pub quantization: String,
    pub vram_required_mb: u64,
    pub compatibility: HardwareFit,
    pub download_url: String,
}

#[derive(Deserialize)]
struct HfApiRepo {
    id: String,
    siblings: Option<Vec<HfSibling>>,
}

#[derive(Deserialize)]
struct HfSibling {
    rfilename: String,
}

/// Queries Hugging Face Hub for GGUF model files and calculates VRAM compatibility.
pub async fn search_models(query: &str, limit: usize) -> Result<Vec<HfModelMetadata>, CoreError> {
    let hw = HardwareCapabilities::detect();
    let url = format!(
        "https://huggingface.co/api/models?search={}&filter=gguf&limit={limit}&full=true",
        urlencoding(query)
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| CoreError::infer("infer.http_client_error", false, e.to_string(), None))?;

    match client.get(&url).send().await {
        Ok(res) if res.status().is_success() => {
            if let Ok(repos) = res.json::<Vec<HfApiRepo>>().await {
                let mut results = Vec::new();
                for repo in repos {
                    if let Some(siblings) = repo.siblings {
                        for sib in siblings {
                            if std::path::Path::new(&sib.rfilename)
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
                            {
                                let meta = parse_gguf_metadata(&repo.id, &sib.rfilename, &hw);
                                results.push(meta);
                            }
                        }
                    }
                }
                if !results.is_empty() {
                    return Ok(results);
                }
            }
        }
        _ => {
            // Network fallback or offline mode: return curated ecosystem models matching query
        }
    }

    Ok(get_curated_models(query, &hw))
}

fn parse_gguf_metadata(
    repo_id: &str,
    filename: &str,
    hw: &HardwareCapabilities,
) -> HfModelMetadata {
    let lower = filename.to_lowercase();
    let quantization = if lower.contains("q4_k_m") {
        "Q4_K_M".to_string()
    } else if lower.contains("q8_0") {
        "Q8_0".to_string()
    } else if lower.contains("q5_k_m") {
        "Q5_K_M".to_string()
    } else if lower.contains("q6_k") {
        "Q6_K".to_string()
    } else if lower.contains("q3_k_m") {
        "Q3_K_M".to_string()
    } else {
        "Q4_K".to_string()
    };

    // Approximate size from quantization tier
    let size_bytes: u64 = if quantization == "Q8_0" {
        7_600_000_000
    } else if quantization == "Q5_K_M" {
        5_300_000_000
    } else {
        4_680_000_000
    };

    let vram_required_mb = (size_bytes / (1024 * 1024)) + 512;
    let compatibility = evaluate_compatibility(vram_required_mb, hw);

    let download_url = format!("https://huggingface.co/{repo_id}/resolve/main/{filename}");

    HfModelMetadata {
        repo_id: repo_id.to_string(),
        filename: filename.to_string(),
        file_size_bytes: size_bytes,
        quantization,
        vram_required_mb,
        compatibility,
        download_url,
    }
}

fn evaluate_compatibility(vram_required_mb: u64, hw: &HardwareCapabilities) -> HardwareFit {
    if hw.vulkan_supported && vram_required_mb <= hw.available_vram_mb {
        HardwareFit::FitsInVram
    } else if vram_required_mb <= (hw.available_vram_mb + hw.available_ram_mb) {
        HardwareFit::PartialOffload
    } else {
        HardwareFit::CpuOnly
    }
}

fn get_curated_models(query: &str, hw: &HardwareCapabilities) -> Vec<HfModelMetadata> {
    let query_lower = query.to_lowercase();
    let catalog = vec![
        (
            "Qwen/Qwen2.5-Coder-7B-Instruct-GGUF",
            "qwen2.5-coder-7b-instruct-q4_k_m.gguf",
            4_680_000_000u64,
            "Q4_K_M",
        ),
        (
            "Qwen/Qwen2.5-Coder-7B-Instruct-GGUF",
            "qwen2.5-coder-7b-instruct-q8_0.gguf",
            7_680_000_000u64,
            "Q8_0",
        ),
        (
            "bartowski/Llama-3.2-3B-Instruct-GGUF",
            "llama-3.2-3b-instruct-q4_k_m.gguf",
            2_150_000_000u64,
            "Q4_K_M",
        ),
        (
            "TheBloke/deepseek-coder-6.7b-instruct-GGUF",
            "deepseek-coder-6.7b-instruct.Q4_K_M.gguf",
            4_200_000_000u64,
            "Q4_K_M",
        ),
    ];

    catalog
        .into_iter()
        .filter(|(repo, file, _, _)| {
            query.is_empty()
                || repo.to_lowercase().contains(&query_lower)
                || file.to_lowercase().contains(&query_lower)
        })
        .map(|(repo, file, bytes, quant)| {
            let vram_mb = (bytes / (1024 * 1024)) + 512;
            HfModelMetadata {
                repo_id: repo.to_string(),
                filename: file.to_string(),
                file_size_bytes: bytes,
                quantization: quant.to_string(),
                vram_required_mb: vram_mb,
                compatibility: evaluate_compatibility(vram_mb, hw),
                download_url: format!("https://huggingface.co/{repo}/resolve/main/{file}"),
            }
        })
        .collect()
}

fn urlencoding(input: &str) -> String {
    let mut out = String::new();
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' {
            out.push(b as char);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}
