use crate::cloud::OpenAiClient;
use crate::engine::InferenceEngine;
use crate::local::guard::LocalGuardDecision;
use crate::local::hardware::HardwareCapabilities;
use crate::local::sidecar::LlamaSidecar;
use crate::types::{ChatRequest, TokenStream};
use async_trait::async_trait;
use starship_core::CoreError;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct LocalGgufRunner {
    pub model_id: String,
    pub file_path: Option<PathBuf>,
    pub context_window: usize,
    pub gpu_layers: u32,
    pub hardware: HardwareCapabilities,
    /// Explicit runtime path (tests / advanced installs); falls back to
    /// `STARSHIP_LLAMA_SERVER`, then PATH.
    runtime_path: Option<PathBuf>,
    sidecar: Arc<Mutex<Option<LlamaSidecar>>>,
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
            runtime_path: None,
            sidecar: Arc::new(Mutex::new(None)),
        }
    }

    /// Overrides the `llama-server` binary path (used by tests and custom installs).
    #[must_use]
    pub fn with_runtime_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.runtime_path = Some(path.into());
        self
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

    /// Evaluates live memory headroom and returns a guard decision before attempting
    /// to load weights or allocate VRAM/RAM (GAP-CLOSURE-PLAN.md §15.3).
    #[must_use]
    pub fn validate_live_headroom(
        &self,
        health: Option<&starship_health::HealthSample>,
    ) -> crate::local::guard::LocalGuardDecision {
        let size_mb = self
            .file_path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map_or(4096, |m| m.len() / (1024 * 1024));

        crate::local::guard::LocalInferenceGuard::evaluate_headroom(
            size_mb,
            self.gpu_layers,
            health,
            self.hardware.vulkan_supported,
        )
    }
}

#[async_trait]
impl InferenceEngine for LocalGgufRunner {
    async fn stream_chat(&self, req: ChatRequest) -> Result<TokenStream, CoreError> {
        self.validate_readiness()?;

        if let LocalGuardDecision::TriggerF3Fallback {
            user_message,
            details,
            ..
        } = self.validate_live_headroom(None)
        {
            return Err(CoreError::infer(
                "infer.model_out_of_memory",
                false,
                user_message,
                Some(details),
            ));
        }

        let model_path = self.file_path.as_ref().ok_or_else(|| {
            CoreError::infer(
                "infer.model_not_found",
                false,
                "No local GGUF file configured",
                None,
            )
        })?;
        let runtime = LlamaSidecar::locate_runtime(self.runtime_path.as_deref())?;

        let mut guard = self.sidecar.lock().await;
        if guard.is_none() {
            let sidecar =
                LlamaSidecar::start(&runtime, model_path, self.context_window, self.gpu_layers)
                    .await?;
            *guard = Some(sidecar);
        }
        let sidecar = guard.as_ref().expect("sidecar started");
        let client = OpenAiClient::new(
            format!("{}/v1", sidecar.base_url()),
            None,
            self.model_id.clone(),
            "local_gguf",
        );
        drop(guard);

        // Dropping the returned stream cancels the in-flight HTTP request.
        client.stream_chat(req).await
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
    use futures::StreamExt;
    use std::os::unix::fs::PermissionsExt;

    fn request() -> ChatRequest {
        ChatRequest {
            model: "local-test".to_string(),
            messages: vec![Message::user("Hello")],
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            tools: vec![],
            grammar: None,
            stream: true,
        }
    }

    #[tokio::test]
    async fn test_local_runner_missing_file_error() {
        let runner = LocalGgufRunner::new(
            "local-test",
            Some(PathBuf::from("/nonexistent/path/model.gguf")),
            2048,
            0,
        );
        let Err(err) = runner.stream_chat(request()).await else {
            panic!("missing file must fail");
        };
        assert_eq!(err.code, "infer.model_not_found");
    }

    #[tokio::test]
    async fn test_local_runner_reports_missing_runtime() {
        let dir = tempfile::tempdir().expect("temp dir");
        let model = dir.path().join("tiny.gguf");
        std::fs::write(&model, b"not-a-real-model").expect("write fixture");
        let runner = LocalGgufRunner::new("local-test", Some(model.clone()), 2048, 0)
            .with_runtime_path(dir.path().join("definitely-missing-llama-server"));
        let Err(err) = runner.stream_chat(request()).await else {
            panic!("missing runtime must fail");
        };
        assert_eq!(err.code, "infer.runtime_missing");
        assert!(err.details.is_some());
    }

    #[tokio::test]
    async fn test_local_runner_streams_from_sidecar() {
        let dir = tempfile::tempdir().expect("temp dir");
        let model = dir.path().join("tiny.gguf");
        std::fs::write(&model, b"fixture").expect("write fixture");
        let runtime = dir.path().join("fake-llama-server.py");
        std::fs::write(
            &runtime,
            r#"#!/usr/bin/env python3
import json, sys, http.server
port = int(sys.argv[sys.argv.index("--port") + 1])
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            self.send_response(200); self.end_headers(); self.wfile.write(b"ok")
        else:
            self.send_response(404); self.end_headers()
    def do_POST(self):
        self.rfile.read(int(self.headers.get("Content-Length", "0")))
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()
        for i in range(24):
            payload = json.dumps({"choices": [{"delta": {"content": "t%d " % i}, "finish_reason": None}]})
            self.wfile.write(("data: " + payload + "\n\n").encode())
            self.wfile.flush()
        self.wfile.write(b"data: [DONE]\n\n"); self.wfile.flush()
    def log_message(self, *args):
        pass
http.server.HTTPServer(("127.0.0.1", port), H).serve_forever()
"#,
        )
        .expect("write runtime stub");
        std::fs::set_permissions(&runtime, std::fs::Permissions::from_mode(0o755))
            .expect("chmod runtime stub");

        let runner =
            LocalGgufRunner::new("local-test", Some(model), 2048, 0).with_runtime_path(&runtime);
        let mut stream = runner
            .stream_chat(request())
            .await
            .expect("sidecar must stream");
        let mut received = String::new();
        let mut chunks = 0_usize;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.expect("valid chunk");
            if !chunk.content.is_empty() {
                chunks += 1;
                received.push_str(&chunk.content);
            }
        }
        assert!(chunks >= 16, "expected >= 16 streamed chunks, got {chunks}");
        assert!(received.contains("t23"), "last real chunk missing");
    }

    /// Real-model verification requires a GGUF file and a genuine llama-server.
    /// Run with `STARSHIP_TEST_GGUF=/path/to/tiny.gguf cargo test -p starship-infer -- --ignored test_local_golden_suite_with_real_model`.
    #[tokio::test]
    #[ignore = "requires a real GGUF model and llama-server"]
    async fn test_local_golden_suite_with_real_model() {
        let Ok(path) = std::env::var("STARSHIP_TEST_GGUF") else {
            return;
        };
        let runner = LocalGgufRunner::new("local-test", Some(PathBuf::from(path)), 2048, 0);
        let mut stream = runner.stream_chat(request()).await.expect("real inference");
        let mut count = 0_usize;
        while let Some(chunk) = stream.next().await {
            if !chunk.expect("chunk").content.is_empty() {
                count += 1;
            }
        }
        assert!(count >= 16, "real model must stream at least 16 chunks");
    }
}
