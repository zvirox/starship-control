#[derive(Debug, Clone, Default)]
pub struct ProviderHeaders {
    pub http_referer: Option<String>,
    pub x_title: Option<String>,
}

use crate::cloud::{AnthropicClient, GeminiClient, OpenAiClient};
use crate::engine::InferenceEngine;
use crate::local::LocalGgufRunner;
use crate::types::{ChatRequest, TokenStream};
use starship_core::CoreError;
use std::sync::Arc;

pub struct InferDispatcher;

impl InferDispatcher {
    /// Creates an `InferenceEngine` instance matching the specified provider and configuration.
    ///
    /// # Errors
    /// Returns `infer.provider_unsupported` if the provider type is unknown.
    pub fn resolve_engine(
        provider_type: &str,
        model_id: &str,
        base_url: Option<&str>,
        api_key: Option<&str>,
        file_path: Option<&str>,
        headers: Option<&ProviderHeaders>,
    ) -> Result<Arc<dyn InferenceEngine>, CoreError> {
        let key_string = api_key.map(ToString::to_string);
        let path_buf = file_path.map(std::path::PathBuf::from);

        match provider_type {
            "local_gguf" => {
                let runner = LocalGgufRunner::new(model_id, path_buf, 8192, 33);
                Ok(Arc::new(runner))
            }
            "cloud_openai" => {
                let url = base_url.unwrap_or("https://api.openai.com/v1");
                let client = OpenAiClient::new(url, key_string, model_id, "cloud_openai")
                    .with_headers(headers);
                Ok(Arc::new(client))
            }
            "cloud_openrouter" => {
                let url = base_url.unwrap_or("https://openrouter.ai/api/v1");
                let client = OpenAiClient::new(url, key_string, model_id, "cloud_openrouter")
                    .with_headers(headers);
                Ok(Arc::new(client))
            }
            "cloud_anthropic" => {
                let url = base_url.unwrap_or("https://api.anthropic.com/v1");
                let client = AnthropicClient::new(url, key_string, model_id);
                Ok(Arc::new(client))
            }
            "cloud_gemini" => {
                let url = base_url.unwrap_or("https://generativelanguage.googleapis.com/v1beta");
                let client = GeminiClient::new(url, key_string, model_id);
                Ok(Arc::new(client))
            }
            _ => Err(CoreError::infer(
                "infer.provider_unsupported",
                false,
                format!("Unsupported inference provider type: {provider_type}"),
                Some(serde_json::json!({ "provider": provider_type })),
            )),
        }
    }

    /// Dispatches a chat request directly to the appropriate engine.
    ///
    /// # Errors
    /// Returns `CoreError` if resolving engine fails or streaming errors out.
    pub async fn dispatch_stream(
        provider_type: &str,
        model_id: &str,
        base_url: Option<&str>,
        api_key: Option<&str>,
        file_path: Option<&str>,
        request: ChatRequest,
        headers: Option<&ProviderHeaders>,
    ) -> Result<TokenStream, CoreError> {
        let engine = Self::resolve_engine(
            provider_type,
            model_id,
            base_url,
            api_key,
            file_path,
            headers,
        )?;
        engine.stream_chat(request).await
    }
}
