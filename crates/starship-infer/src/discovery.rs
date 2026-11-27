//! Provider model discovery (Phase 17, 17.20–17.21).
//!
//! Calls each provider's public `/models` endpoint, never logs the key, and
//! returns normalized model metadata for the chat picker.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starship_core::CoreError;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredModel {
    pub id: String,
    pub display_name: String,
    pub context_window: i64,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub input_price: f64,
    pub output_price: f64,
    pub raw: serde_json::Value,
}

#[async_trait]
pub trait ModelDiscovery: Send + Sync {
    async fn list_models(&self) -> Result<Vec<DiscoveredModel>, CoreError>;
}

/// Normalizes an OpenAI-compatible `/models` response (`OpenAI`, `OpenRouter`,
/// `llama-server`, custom endpoints). `OpenRouter`'s richer fields are used.
pub fn parse_openai_models(body: &str) -> Result<Vec<DiscoveredModel>, CoreError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        CoreError::infer(
            "infer.stream_parse_error",
            false,
            format!("Invalid models response: {e}"),
            None,
        )
    })?;
    let entries = value.get("data").and_then(|d| d.as_array());
    let Some(entries) = entries else {
        return Err(CoreError::infer(
            "infer.stream_parse_error",
            false,
            "Models response has no data array",
            None,
        ));
    };
    Ok(entries
        .iter()
        .filter_map(|entry| {
            let id = entry.get("id")?.as_str()?.to_string();
            let params = entry
                .get("supported_parameters")
                .and_then(|p| p.as_array())
                .cloned()
                .unwrap_or_default();
            let supports_tools = params
                .iter()
                .any(|p| p.as_str().is_some_and(|s| s == "tools"));
            let price = |field: &str| -> f64 {
                entry
                    .get("pricing")
                    .and_then(|p| p.get(field))
                    .and_then(serde_json::Value::as_str)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0)
            };
            Some(DiscoveredModel {
                display_name: entry
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(&id)
                    .to_string(),
                id,
                context_window: entry
                    .get("context_length")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0),
                supports_tools,
                supports_streaming: true,
                input_price: price("prompt"),
                output_price: price("completion"),
                raw: entry.clone(),
            })
        })
        .collect())
}

pub fn parse_anthropic_models(body: &str) -> Result<Vec<DiscoveredModel>, CoreError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        CoreError::infer(
            "infer.stream_parse_error",
            false,
            format!("Invalid Anthropic models response: {e}"),
            None,
        )
    })?;
    let entries: &[serde_json::Value] = value
        .get("data")
        .and_then(|d| d.as_array())
        .map_or(&[], Vec::as_slice);
    Ok(entries
        .iter()
        .filter_map(|entry| {
            let id = entry.get("id")?.as_str()?.to_string();
            Some(DiscoveredModel {
                display_name: entry
                    .get("display_name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(&id)
                    .to_string(),
                id,
                context_window: entry
                    .get("context_window")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0),
                supports_tools: entry
                    .get("tools")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                supports_streaming: true,
                input_price: 0.0,
                output_price: 0.0,
                raw: entry.clone(),
            })
        })
        .collect())
}

pub fn parse_gemini_models(body: &str) -> Result<Vec<DiscoveredModel>, CoreError> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|e| {
        CoreError::infer(
            "infer.stream_parse_error",
            false,
            format!("Invalid Gemini models response: {e}"),
            None,
        )
    })?;
    let entries: &[serde_json::Value] = value
        .get("models")
        .and_then(|m| m.as_array())
        .map_or(&[], Vec::as_slice);
    Ok(entries
        .iter()
        .filter(|entry| {
            entry
                .get("supportedGenerationMethods")
                .and_then(|m| m.as_array())
                .is_some_and(|methods| {
                    methods
                        .iter()
                        .any(|m| m.as_str().is_some_and(|s| s == "generateContent"))
                })
        })
        .filter_map(|entry| {
            let full = entry.get("name")?.as_str()?;
            let id = full.strip_prefix("models/").unwrap_or(full).to_string();
            Some(DiscoveredModel {
                display_name: entry
                    .get("displayName")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or(&id)
                    .to_string(),
                id,
                context_window: entry
                    .get("inputTokenLimit")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0),
                supports_tools: true,
                supports_streaming: true,
                input_price: 0.0,
                output_price: 0.0,
                raw: entry.clone(),
            })
        })
        .collect())
}

#[derive(Debug, Clone)]
pub struct ProviderDiscovery {
    pub provider_type: String,
    pub base_url: String,
    pub api_key: Option<String>,
}

impl ProviderDiscovery {
    #[must_use]
    pub fn new(
        provider_type: impl Into<String>,
        base_url: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            provider_type: provider_type.into(),
            base_url: base_url.into(),
            api_key,
        }
    }

    async fn fetch(&self) -> Result<String, CoreError> {
        let url = format!("{}/models", self.base_url.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| {
                CoreError::infer(
                    "infer.cloud_endpoint_unreachable",
                    false,
                    format!("Cannot build discovery client: {e}"),
                    None,
                )
            })?;
        for attempt in 0..2 {
            let mut request = client.get(&url);
            // The key is only ever placed in headers, never in a URL or log.
            match self.provider_type.as_str() {
                "cloud_anthropic" => {
                    if let Some(key) = &self.api_key {
                        request = request.header("x-api-key", key);
                    }
                    request = request.header("anthropic-version", "2023-06-01");
                }
                "cloud_gemini" => {
                    if let Some(key) = &self.api_key {
                        request = request.header("x-goog-api-key", key);
                    }
                }
                _ => {
                    if let Some(key) = &self.api_key {
                        request = request.header("Authorization", format!("Bearer {key}"));
                    }
                }
            }
            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        return response.text().await.map_err(|e| {
                            CoreError::infer(
                                "infer.cloud_endpoint_unreachable",
                                true,
                                format!("Cannot read models response: {e}"),
                                None,
                            )
                        });
                    }
                    if status == reqwest::StatusCode::UNAUTHORIZED
                        || status == reqwest::StatusCode::FORBIDDEN
                    {
                        return Err(CoreError::infer(
                            "infer.cloud_auth_failed",
                            false,
                            format!("Models endpoint rejected the key (HTTP {status})"),
                            Some(serde_json::json!({ "status": status.as_u16() })),
                        ));
                    }
                    if status.is_server_error()
                        || status == reqwest::StatusCode::TOO_MANY_REQUESTS && attempt == 0
                    {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        continue;
                    }
                    return Err(CoreError::infer(
                        "infer.cloud_endpoint_unreachable",
                        false,
                        format!("Models endpoint returned HTTP {status}"),
                        Some(serde_json::json!({ "status": status.as_u16() })),
                    ));
                }
                Err(_e) if attempt == 0 => {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
                Err(_) => {
                    return Err(CoreError::infer(
                        "infer.cloud_endpoint_unreachable",
                        true,
                        "Models endpoint is unreachable",
                        None,
                    ));
                }
            }
        }
        Err(CoreError::infer(
            "infer.cloud_endpoint_unreachable",
            true,
            "Models endpoint did not respond after retry",
            None,
        ))
    }
}

#[async_trait]
impl ModelDiscovery for ProviderDiscovery {
    async fn list_models(&self) -> Result<Vec<DiscoveredModel>, CoreError> {
        let body = self.fetch().await?;
        match self.provider_type.as_str() {
            "cloud_anthropic" => parse_anthropic_models(&body),
            "cloud_gemini" => parse_gemini_models(&body),
            _ => parse_openai_models(&body),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn test_parsers_normalize_all_three_providers() {
        let openai = r#"{"data":[{"id":"openrouter/qwen","name":"Qwen","context_length":32000,"pricing":{"prompt":"0.000001","completion":"0.000002"},"supported_parameters":["tools","temperature"]}]}"#;
        let models = parse_openai_models(openai).expect("openai models");
        assert_eq!(models[0].id, "openrouter/qwen");
        assert_eq!(models[0].context_window, 32_000);
        assert!(models[0].supports_tools);
        assert!(models[0].input_price > 0.0);

        let anthropic = r#"{"data":[{"id":"claude-3","display_name":"Claude 3","context_window":200000,"tools":true}],"has_more":false}"#;
        let models = parse_anthropic_models(anthropic).expect("anthropic models");
        assert_eq!(models[0].display_name, "Claude 3");
        assert!(models[0].supports_tools);

        let gemini = r#"{"models":[{"name":"models/gemini-2.0","displayName":"Gemini 2","inputTokenLimit":1000000,"supportedGenerationMethods":["generateContent"]},{"name":"models/embed","supportedGenerationMethods":["embedContent"]}]}"#;
        let models = parse_gemini_models(gemini).expect("gemini models");
        assert_eq!(models.len(), 1, "non-generative models must be filtered");
        assert_eq!(models[0].id, "gemini-2.0");
    }

    #[tokio::test]
    async fn test_discovery_sends_key_only_in_authorization_header() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stub");
        let port = listener.local_addr().expect("port").port();
        let body = r#"{"data":[{"id":"stub-model"}]}"#;
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 4096];
            let read = socket.read(&mut buffer).await.expect("read request");
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
            request
        });
        let discovery = ProviderDiscovery::new(
            "cloud_openrouter",
            format!("http://127.0.0.1:{port}"),
            Some("sk-canary".to_string()),
        );
        let models = discovery.list_models().await.expect("discovery");
        assert_eq!(models[0].id, "stub-model");
        let request = server.await.expect("stub task");
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer sk-canary")
        );
        assert!(!request.contains("sk-canary?"));
    }
}
