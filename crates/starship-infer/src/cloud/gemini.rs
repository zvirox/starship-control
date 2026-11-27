use crate::engine::InferenceEngine;
use crate::types::{ChatChunk, ChatRequest, TokenStream, ToolCall};
use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use starship_core::CoreError;

#[derive(Debug, Clone)]
pub struct GeminiClient {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model_id: String,
    http: reqwest::Client,
}

impl GeminiClient {
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
struct GeminiPayload<'a> {
    contents: Vec<GeminiContent<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiSystemInstruction<'a>>,
}

#[derive(Serialize)]
struct GeminiSystemInstruction<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize)]
struct GeminiContent<'a> {
    role: &'a str,
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContentResponse>,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Option<Vec<GeminiPartResponse>>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    text: Option<String>,
    #[serde(rename = "functionCall")]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: Option<serde_json::Value>,
}

#[async_trait]
impl InferenceEngine for GeminiClient {
    async fn stream_chat(&self, req: ChatRequest) -> Result<TokenStream, CoreError> {
        let key_param = self
            .api_key
            .as_ref()
            .map_or_else(String::new, |k| format!("&key={k}"));

        let url = format!(
            "{}/models/{}:streamGenerateContent?alt=sse{}",
            self.base_url.trim_end_matches('/'),
            self.model_id,
            key_param
        );

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let mut contents = Vec::new();
        for m in &req.messages {
            let role = match m.role {
                crate::types::Role::Assistant => "model",
                crate::types::Role::User
                | crate::types::Role::Tool
                | crate::types::Role::System => "user",
            };
            contents.push(GeminiContent {
                role,
                parts: vec![GeminiPart { text: &m.content }],
            });
        }

        let system_instruction = req
            .system_prompt
            .as_deref()
            .map(|text| GeminiSystemInstruction {
                parts: vec![GeminiPart { text }],
            });

        let payload = GeminiPayload {
            contents,
            system_instruction,
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
                    format!("Failed to reach Gemini endpoint at {url}: {e}"),
                    None,
                )
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(CoreError::infer(
                "infer.cloud_auth_failed",
                status.as_u16() == 429 || status.is_server_error(),
                format!("Gemini API error ({status}): {text}"),
                Some(serde_json::json!({ "status": status.as_u16() })),
            ));
        }

        let event_stream = response.bytes_stream().eventsource();
        let mapped = event_stream.filter_map(|event_res| async move {
            match event_res {
                Ok(event) => match serde_json::from_str::<GeminiResponse>(&event.data) {
                    Ok(resp) => {
                        if let Some(candidate) = resp.candidates.and_then(|mut c| c.pop()) {
                            let mut text_content = String::new();
                            let mut tool_calls = Vec::new();
                            if let Some(parts) = candidate.content.and_then(|c| c.parts) {
                                for p in parts {
                                    if let Some(t) = p.text {
                                        text_content.push_str(&t);
                                    }
                                    if let Some(call) = p.function_call {
                                        tool_calls.push(ToolCall {
                                            id: call.name.clone(),
                                            name: call.name,
                                            arguments: call
                                                .args
                                                .unwrap_or_else(|| serde_json::json!({})),
                                        });
                                    }
                                }
                            }

                            let is_final = candidate.finish_reason.is_some();
                            Some(Ok(ChatChunk {
                                content: text_content,
                                thought: None,
                                is_final,
                                finish_reason: candidate.finish_reason,
                                tool_calls: if tool_calls.is_empty() {
                                    None
                                } else {
                                    Some(tool_calls)
                                },
                            }))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(CoreError::infer(
                        "infer.stream_parse_error",
                        false,
                        format!("Failed to parse Gemini SSE: {e}"),
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
        "cloud_gemini"
    }
}
