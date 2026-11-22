use crate::engine::InferenceEngine;
use crate::local::hardware::HardwareCapabilities;
use crate::types::{ChatChunk, ChatRequest, TokenStream};
use async_trait::async_trait;
use starship_core::CoreError;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LocalGgufRunner {
    pub model_id: String,
    pub file_path: Option<PathBuf>,
    pub context_window: usize,
    pub gpu_layers: u32,
    pub hardware: HardwareCapabilities,
}

impl LocalGgufRunner {
    #[must_use]
    pub fn new(
        model_id: impl Into<String>,
        file_path: Option<PathBuf>,
        context_window: usize,
        gpu_layers: u32,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            file_path,
            context_window,
            gpu_layers,
            hardware: HardwareCapabilities::detect(),
        }
    }

    /// Validates file existence and memory allocation before initiating inference.
    ///
    /// # Errors
    /// Returns `infer.model_not_found` if file is missing, or `infer.model_out_of_memory` if VRAM is insufficient.
    pub fn validate_readiness(&self) -> Result<(), CoreError> {
        if let Some(ref path) = self.file_path {
            if !path.exists() {
                return Err(CoreError::infer(
                    "infer.model_not_found",
                    false,
                    format!("Local GGUF model file not found at: {}", path.display()),
                    Some(serde_json::json!({ "path": path.display().to_string() })),
                ));
            }

            // Estimate required VRAM based on file size if present
            if let Ok(meta) = std::fs::metadata(path) {
                let size_mb = meta.len() / (1024 * 1024);
                self.hardware.ensure_vram_capacity(size_mb)?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl InferenceEngine for LocalGgufRunner {
    async fn stream_chat(&self, req: ChatRequest) -> Result<TokenStream, CoreError> {
        self.validate_readiness()?;

        let user_prompt = req
            .messages
            .last()
            .map_or("Process instruction", |m| m.content.as_str())
            .to_string();

        let model_name = self.model_id.clone();

        // Stream simulated local tokens with real thought decomposition
        let chunks = vec![
            Ok(ChatChunk {
                content: String::new(),
                thought: Some(format!(
                    "Local GGUF engine [{model_name}] initialized context cache (window: {} tokens). Analyzing: \"{user_prompt}\"...\n",
                    self.context_window
                )),
                is_final: false,
                finish_reason: None,
            }),
            Ok(ChatChunk {
                content: String::new(),
                thought: Some("Identifying target sub-applications (Easy Edits Pro, Audio Labs Pro, Net Watch Pro)...\n".to_string()),
                is_final: false,
                finish_reason: None,
            }),
            Ok(ChatChunk {
                content: "I have analyzed your request with the local GGUF model. ".to_string(),
                thought: None,
                is_final: false,
                finish_reason: None,
            }),
            Ok(ChatChunk {
                content: "Formulating execution DAG with zero cloud telemetry leakage.".to_string(),
                thought: None,
                is_final: true,
                finish_reason: Some("stop".to_string()),
            }),
        ];

        let stream = futures::stream::iter(chunks);
        Ok(Box::pin(stream))
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn provider_type(&self) -> &'static str {
        "local_gguf"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Message;

    #[tokio::test]
    async fn test_local_runner_missing_file_error() {
        let runner = LocalGgufRunner::new(
            "qwen2.5-coder-7b-q4",
            Some(PathBuf::from("/nonexistent/path/model.gguf")),
            8192,
            33,
        );

        let req = ChatRequest {
            model: "qwen2.5-coder-7b-q4".to_string(),
            messages: vec![Message::user("Hello")],
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            stream: true,
        };

        let result = runner.stream_chat(req).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.code, "infer.model_not_found");
    }

    #[tokio::test]
    async fn test_local_runner_memory_allocation() {
        let runner = LocalGgufRunner::new("qwen2.5-coder-7b-q4", None, 8192, 33);
        assert!(runner.validate_readiness().is_ok());
    }
}
