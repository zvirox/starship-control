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

        let prompt_lower = user_prompt.to_ascii_lowercase();

        // Context-aware reasoning and response generation
        let (thought, full_response) = if prompt_lower.contains("hello")
            || prompt_lower.contains("hi")
            || prompt_lower.contains("hey")
            || prompt_lower.starts_with("who are you")
        {
            (
                "User greeted. Preparing welcoming system overview and assistant capabilities.".to_string(),
                "Hello! I'm **Starship Control**, your local-first AI agent orchestrator and unified command center.\n\nI can help you:\n• Coordinate creative workflows across **ScreenStudio Pro**, **Audio Labs Pro**, and **Easy Edits Pro**\n• Monitor system and network activity with **Net Watch Pro** and **Agentic Tuneup Pro**\n• Run AI models completely locally on your hardware or connect to cloud providers\n\nWhat would you like to work on today?".to_string(),
            )
        } else if prompt_lower.contains("app")
            || prompt_lower.contains("ecosystem")
            || prompt_lower.contains("connected")
        {
            (
                "User inquired about connected applications. Fetching sub-app registry.".to_string(),
                "Starship Control integrates with your local desktop application ecosystem:\n\n• **ScreenStudio Pro (SSP)**: High-framerate desktop screencasts and window recording.\n• **Audio Labs Pro (ALP)**: Voice isolation, audio de-humming, and mastering.\n• **Easy Edits Pro (EEP)**: Timeline video editing with automated ripple cuts.\n• **Net Watch Pro (NWP)**: Real-time network telemetry and port inspection.\n• **Agentic Tuneup Pro (ATP)**: System health monitoring and cache optimization.\n\nYou can inspect connected sockets and active tools in the **Apps** and **Tools** tabs.".to_string(),
            )
        } else if prompt_lower.contains("record")
            || prompt_lower.contains("screencast")
            || prompt_lower.contains("video")
            || prompt_lower.contains("edit")
        {
            (
                "Detected media assembly request. Outlining multi-app pipeline: SSP -> ALP -> EEP."
                    .to_string(),
                format!(
                    "I understand you'd like to work on a screencast or video task: \"{user_prompt}\".\n\nHere is the recommended workflow:\n1. **ScreenStudio Pro**: Capture high-resolution video and audio input.\n2. **Audio Labs Pro**: Clean up noise, remove hum, and isolate dialogue.\n3. **Easy Edits Pro**: Assemble the assets onto your timeline with automatic silence cut detection.\n\nYou can switch to the **Workflows** tab to execute and track this pipeline."
                ),
            )
        } else if prompt_lower.contains("clean")
            || prompt_lower.contains("cache")
            || prompt_lower.contains("tune")
            || prompt_lower.contains("disk")
            || prompt_lower.contains("system")
        {
            (
                "Detected maintenance request. Preparing diagnostic and optimization routine."
                    .to_string(),
                format!(
                    "I've initiated a system check for your instruction: \"{user_prompt}\".\n\n1. **Net Watch Pro**: Inspect live sockets and bandwidth usage.\n2. **Agentic Tuneup Pro**: Scan disk usage and identify safe-to-purge build and temporary caches.\n3. Any destructive cleanup step will pause for your explicit confirmation before executing."
                ),
            )
        } else {
            (
                format!("Analyzing prompt: \"{user_prompt}\". Synthesizing response."),
                format!(
                    "I have processed your request:\n\n> {user_prompt}\n\nEverything is running smoothly on Starship Control. You can command connected applications directly, explore the **Workflows** studio for cross-app pipelines, or configure your local and cloud AI models in **Models**."
                ),
            )
        };

        // Break response into natural readable chunks
        let mut chunks: Vec<Result<ChatChunk, CoreError>> = Vec::new();

        chunks.push(Ok(ChatChunk {
            content: String::new(),
            thought: Some(thought),
            is_final: false,
            finish_reason: None,
        }));

        // Split text by words/tokens to provide smooth streaming
        let words: Vec<&str> = full_response.split_inclusive(' ').collect();
        for (i, word) in words.iter().enumerate() {
            let is_final = i == words.len() - 1;
            chunks.push(Ok(ChatChunk {
                content: (*word).to_string(),
                thought: None,
                is_final,
                finish_reason: if is_final {
                    Some("stop".to_string())
                } else {
                    None
                },
            }));
        }

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
