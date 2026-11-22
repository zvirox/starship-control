use crate::engine::InferenceEngine;
use crate::types::{ChatChunk, ChatRequest, TokenStream};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use starship_core::CoreError;

#[derive(Debug, Clone)]
pub struct AnthropicClient {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model_id: String,
    http: reqwest::Client,
}

impl AnthropicClient {
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
        model_id: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key,
            model_id: model_id.into(),
            http: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct AnthropicPayload<'a> {
    model: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: AnthropicDelta },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: AnthropicMessageDelta },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct AnthropicMessageDelta {
    stop_reason: Option<String>,
}

#[async_trait]
impl InferenceEngine for AnthropicClient {
    async fn stream_chat(&self, req: ChatRequest) -> Result<TokenStream, CoreError> {
        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        if let Some(ref key) = self.api_key
            && let Ok(val) = HeaderValue::from_str(key)
        {
            headers.insert("x-api-key", val);
        }

        let mut system_text = req.system_prompt.as_deref();
        let mut messages = Vec::new();

        for m in &req.messages {
            match m.role {
                crate::types::Role::System => {
                    if system_text.is_none() {
                        system_text = Some(&m.content);
                    }
                }
                crate::types::Role::User | crate::types::Role::Tool => {
                    messages.push(AnthropicMessage {
                        role: "user",
                        content: &m.content,
                    });
                }
                crate::types::Role::Assistant => {
                    messages.push(AnthropicMessage {
                        role: "assistant",
                        content: &m.content,
                    });
                }
            }
        }

        let payload = AnthropicPayload {
            model: &self.model_id,
            messages,
            max_tokens: req.max_tokens.unwrap_or(4096),
            stream: true,
            system: system_text,
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
                    format!("Failed to reach Anthropic endpoint at {url}: {e}"),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(CoreError::infer(
                "infer.cloud_auth_failed",
                status.as_u16() == 429 || status.is_server_error(),
                format!("Anthropic API error ({status}): {text}"),
                Some(serde_json::json!({ "status": status.as_u16() })),
            ));
        }

        let event_stream = response.bytes_stream().eventsource();
        let mapped = event_stream.filter_map(|event_res| async move {
            match event_res {
                Ok(event) => match serde_json::from_str::<AnthropicEvent>(&event.data) {
                    Ok(AnthropicEvent::ContentBlockDelta { delta }) => match delta {
                        AnthropicDelta::TextDelta { text } => Some(Ok(ChatChunk {
                            content: text,
                            thought: None,
                            is_final: false,
                            finish_reason: None,
                        })),
                        AnthropicDelta::ThinkingDelta { thinking } => Some(Ok(ChatChunk {
                            content: String::new(),
                            thought: Some(thinking),
                            is_final: false,
                            finish_reason: None,
                        })),
                        AnthropicDelta::Other => None,
                    },
                    Ok(AnthropicEvent::MessageDelta { delta }) => Some(Ok(ChatChunk {
                        content: String::new(),
                        thought: None,
                        is_final: delta.stop_reason.is_some(),
                        finish_reason: delta.stop_reason,
                    })),
                    Ok(AnthropicEvent::MessageStop) => Some(Ok(ChatChunk {
                        content: String::new(),
                        thought: None,
                        is_final: true,
                        finish_reason: Some("stop".to_string()),
                    })),
                    Ok(AnthropicEvent::Unknown) => None,
                    Err(e) => Some(Err(CoreError::infer(
                        "infer.stream_parse_error",
                        false,
                        format!("Failed to parse Anthropic SSE: {e}"),
                        Some(serde_json::json!({ "raw": event.data })),
                    ))),
                },
                Err(e) => Some(Err(CoreError::infer(
                    "infer.stream_error",
                    true,
                    format!("SSE stream connection error: {e}"),
                    None,
                ))),
            }
        });

        Ok(Box::pin(mapped))
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn provider_type(&self) -> &'static str {
        "cloud_anthropic"
    }
}
