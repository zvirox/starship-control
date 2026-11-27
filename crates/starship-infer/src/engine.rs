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
            tool_calls: None,
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
                tool_calls: None,
            }),
            Ok(ChatChunk {
                content: format!("Starship Control acknowledged: {prompt}. "),
                thought: None,
                is_final: false,
                finish_reason: None,
                tool_calls: None,
            }),
            Ok(ChatChunk {
                content: "Executing plan step 1 autonomously.".to_string(),
                thought: None,
                is_final: true,
                finish_reason: Some("stop".to_string()),
                tool_calls: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChatChunk, Message, Role};
    use futures::StreamExt;

    /// Deterministic engine that echoes the full context it receives, used to
    /// prove that two-turn memory reaches the model (Phase 17, 17.12).
    #[derive(Debug, Clone)]
    struct ContextEchoEngine;

    #[async_trait::async_trait]
    impl InferenceEngine for ContextEchoEngine {
        async fn stream_chat(&self, req: ChatRequest) -> Result<TokenStream, CoreError> {
            let context = req
                .messages
                .iter()
                .map(|message| message.content.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            Ok(Box::pin(futures::stream::iter(vec![Ok(ChatChunk {
                content: context,
                thought: None,
                is_final: true,
                finish_reason: Some("stop".to_string()),
                tool_calls: None,
            })])))
        }

        fn model_id(&self) -> &'static str {
            "echo"
        }

        fn provider_type(&self) -> &'static str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_echo_engine_sees_full_two_turn_context() {
        let req = ChatRequest {
            model: "mock".to_string(),
            messages: vec![
                Message {
                    role: Role::User,
                    content: "My name is Zvirox".to_string(),
                    thought: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                Message::assistant("Hello Zvirox"),
                Message::user("What is my name?"),
            ],
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            grammar: None,
            stream: true,
        };
        let mut stream = ContextEchoEngine
            .stream_chat(req)
            .await
            .expect("echo stream");
        let chunk = stream
            .next()
            .await
            .expect("one chunk")
            .expect("valid chunk");
        assert!(chunk.content.contains("My name is Zvirox"));
        assert!(chunk.content.contains("What is my name?"));
    }
}
