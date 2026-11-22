pub mod cloud;
pub mod dispatcher;
pub mod engine;
pub mod local;
pub mod types;

pub use dispatcher::InferDispatcher;
pub use engine::{InferenceEngine, MockInferenceEngine};
pub use types::{ChatChunk, ChatRequest, Message, Role, TokenStream, ToolCall, ToolDefinition};

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_mock_inference_engine_stream() {
        let engine = MockInferenceEngine::default();
        let req = ChatRequest {
            model: "mock".to_string(),
            messages: vec![Message::user("hello")],
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            stream: true,
        };

        let mut stream = engine.stream_chat(req).await.expect("stream_chat");

        let chunk1 = stream.next().await.expect("chunk 1").expect("chunk 1 ok");
        assert_eq!(chunk1.content, "");
        assert!(chunk1.thought.is_some());
        assert!(!chunk1.is_final);

        let chunk2 = stream.next().await.expect("chunk 2").expect("chunk 2 ok");
        assert!(chunk2.content.contains("Starship Control acknowledged"));
        assert!(!chunk2.is_final);

        let chunk3 = stream.next().await.expect("chunk 3").expect("chunk 3 ok");
        assert!(chunk3.is_final);

        assert!(stream.next().await.is_none());
    }
}
