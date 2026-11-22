use crate::engine::InferenceEngine;
use crate::types::{ChatChunk, ChatRequest, TokenStream};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use starship_core::CoreError;

#[derive(Debug, Clone)]
pub struct OpenAiClient {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model_id: String,
    pub provider_type: String,
    http: reqwest::Client,
}

impl OpenAiClient {
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        model_id: impl Into<String>,
        provider_type: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key,
            model_id: model_id.into(),
            provider_type: provider_type.into(),
            http: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct OpenAiChatPayload<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OpenAiStreamResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Default)]
struct OpenAiDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[async_trait]
impl InferenceEngine for OpenAiClient {
    async fn stream_chat(&self, req: ChatRequest) -> Result<TokenStream, CoreError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(ref key) = self.api_key {
            let auth_val = format!("Bearer {key}");
            if let Ok(header_val) = HeaderValue::from_str(&auth_val) {
                headers.insert(AUTHORIZATION, header_val);
            }
        }

        let messages: Vec<OpenAiMessage<'_>> = req
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    crate::types::Role::System => "system",
                    crate::types::Role::User => "user",
                    crate::types::Role::Assistant => "assistant",
                    crate::types::Role::Tool => "tool",
                };
                OpenAiMessage {
                    role,
                    content: &m.content,
                }
            })
            .collect();

        let payload = OpenAiChatPayload {
            model: &self.model_id,
            messages,
            stream: true,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        };

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                CoreError::infer(
                    "infer.cloud_endpoint_unreachable",
                    true,
                    format!("Failed to reach OpenAI-compatible endpoint at {url}: {e}"),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(CoreError::infer(
                "infer.cloud_auth_failed",
                status.as_u16() == 429 || status.is_server_error(),
                format!("OpenAI API error ({status}): {text}"),
                Some(serde_json::json!({ "status": status.as_u16() })),
            ));
        }

        let event_stream = response.bytes_stream().eventsource();
        let mapped_stream = event_stream.map(|event_result| match event_result {
            Ok(event) => {
                let data = event.data;
                if data == "[DONE]" {
                    return Ok(ChatChunk {
                        content: String::new(),
                        thought: None,
                        is_final: true,
                        finish_reason: Some("stop".to_string()),
                    });
                }

                match serde_json::from_str::<OpenAiStreamResponse>(&data) {
                    Ok(resp) => {
                        let choice = resp.choices.into_iter().next();
                        let (content, thought, finish_reason) =
                            choice.map_or((String::new(), None, None), |c| {
                                (
                                    c.delta.content.unwrap_or_default(),
                                    c.delta.reasoning_content,
                                    c.finish_reason,
                                )
                            });

                        let is_final = finish_reason.is_some();
                        Ok(ChatChunk {
                            content,
                            thought,
                            is_final,
                            finish_reason,
                        })
                    }
                    Err(e) => Err(CoreError::infer(
                        "infer.stream_parse_error",
                        false,
                        format!("Failed to parse SSE payload: {e}"),
                        Some(serde_json::json!({ "raw": data })),
                    )),
                }
            }
            Err(e) => Err(CoreError::infer(
                "infer.stream_error",
                true,
                format!("SSE stream connection error: {e}"),
                None,
            )),
        });

        Ok(Box::pin(mapped_stream))
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn provider_type(&self) -> &str {
        &self.provider_type
    }
}
