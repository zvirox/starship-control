use crate::types::{ChatChunk, ChatRequest, TokenStream};
use async_trait::async_trait;
use futures::StreamExt;
use starship_core::CoreError;

#[async_trait]
pub trait InferenceEngine: Send + Sync {
    /// Streams tokens and reasoning traces asynchronously.
    async fn stream_chat(&self, req: ChatRequest) -> Result<TokenStream, CoreError>;

    /// Executes inference to completion and aggregates chunks into a single response.
    async fn complete_chat(&self, req: ChatRequest) -> Result<ChatChunk, CoreError> {
        let mut stream = self.stream_chat(req).await?;
        let mut total_content = String::new();
        let mut total_thought = String::new();
        let mut finish_reason = None;

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res?;
            total_content.push_str(&chunk.content);
            if let Some(thought) = chunk.thought {
                total_thought.push_str(&thought);
            }
            if chunk.finish_reason.is_some() {
                finish_reason = chunk.finish_reason;
            }
        }

        Ok(ChatChunk {
            content: total_content,
            thought: if total_thought.is_empty() {
                None
            } else {
                Some(total_thought)
            },
            is_final: true,
            finish_reason,
        })
    }

    fn model_id(&self) -> &str;
    fn provider_type(&self) -> &str;
}

/// Deterministic mock inference engine for testing and vertical slice verification.
#[derive(Debug, Clone)]
pub struct MockInferenceEngine {
    pub model_id: String,
    pub provider_type: String,
}

impl Default for MockInferenceEngine {
    fn default() -> Self {
        Self {
            model_id: "mock-model".to_string(),
            provider_type: "mock".to_string(),
        }
    }
}

#[async_trait]
impl InferenceEngine for MockInferenceEngine {
    async fn stream_chat(&self, req: ChatRequest) -> Result<TokenStream, CoreError> {
        let prompt = req.messages.last().map_or("Hello", |m| m.content.as_str());

        let chunks = vec![
            Ok(ChatChunk {
                content: String::new(),
                thought: Some(
                    "Decomposing workflow and inspecting application state...".to_string(),
                ),
                is_final: false,
                finish_reason: None,
            }),
            Ok(ChatChunk {
                content: format!("Starship Control acknowledged: {prompt}. "),
                thought: None,
                is_final: false,
                finish_reason: None,
            }),
            Ok(ChatChunk {
                content: "Executing plan step 1 autonomously.".to_string(),
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

    fn provider_type(&self) -> &str {
        &self.provider_type
    }
}
