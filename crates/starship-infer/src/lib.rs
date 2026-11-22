use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatChunk {
    pub content: String,
    pub thought: Option<String>,
    pub is_final: bool,
}

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, CoreError>> + Send>>;

#[async_trait]
pub trait InferenceEngine: Send + Sync {
    async fn stream_chat(&self, prompt: &str) -> Result<TokenStream, CoreError>;
}

/// Mock inference engine for deterministic verification.
#[derive(Debug, Default)]
pub struct MockInferenceEngine;

#[async_trait]
impl InferenceEngine for MockInferenceEngine {
    async fn stream_chat(&self, _prompt: &str) -> Result<TokenStream, CoreError> {
        let chunks = vec![
            Ok(ChatChunk {
                content: "Starship".to_string(),
                thought: Some("Analyzing instruction...".to_string()),
                is_final: false,
            }),
            Ok(ChatChunk {
                content: " online.".to_string(),
                thought: None,
                is_final: true,
            }),
        ];

        let stream = futures::stream::iter(chunks);
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_inference_engine_stream() {
        let engine = MockInferenceEngine;
        let mut stream = engine.stream_chat("hello").await.expect("stream_chat");

        let chunk1 = stream.next().await.expect("chunk 1").expect("chunk 1 ok");
        assert_eq!(chunk1.content, "Starship");
        assert_eq!(chunk1.thought.as_deref(), Some("Analyzing instruction..."));
        assert!(!chunk1.is_final);

        let chunk2 = stream.next().await.expect("chunk 2").expect("chunk 2 ok");
        assert_eq!(chunk2.content, " online.");
        assert!(chunk2.is_final);

        assert!(stream.next().await.is_none());
    }
}
