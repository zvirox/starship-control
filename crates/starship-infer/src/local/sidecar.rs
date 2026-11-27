//! Managed `llama-server` sidecar for real local GGUF inference (Phase 17, 17.10).
//!
//! Path A from `PHASE-17-FIX-PLAN.md`: spawn one OpenAI-compatible
//! `llama-server` process per model, wait for `/health`, and stream through the
//! existing OpenAI-compatible client. The process is killed on drop so no
//! runtime is left behind.

use serde_json::json;
use sha2::Digest;
use starship_core::CoreError;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// A running `llama-server` process bound to a loopback port.
pub struct LlamaSidecar {
    child: tokio::sync::Mutex<Option<tokio::process::Child>>,
    pub port: u16,
}

impl std::fmt::Debug for LlamaSidecar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlamaSidecar")
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

impl LlamaSidecar {
    /// Finds the runtime binary: explicit path, `STARSHIP_LLAMA_SERVER`, then PATH.
    /// Intentionally read-only: downloads are out of scope for this task.
    pub fn locate_runtime(explicit: Option<&Path>) -> Result<PathBuf, CoreError> {
        if let Some(path) = explicit
            && path.exists()
        {
            return Ok(path.to_path_buf());
        }
        if let Ok(value) = std::env::var("STARSHIP_LLAMA_SERVER")
            && Path::new(&value).exists()
        {
            return Ok(PathBuf::from(value));
        }
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                let candidate = dir.join("llama-server");
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }
        Err(CoreError::infer(
            "infer.runtime_missing",
            false,
            "llama-server runtime not found",
            Some(json!({
                "hint": "Install llama.cpp and set STARSHIP_LLAMA_SERVER to its llama-server binary",
                "env": "STARSHIP_LLAMA_SERVER"
            })),
        ))
    }

    /// Verifies the runtime digest when a SHA-256 is configured. Pinning is
    /// opt-in via `STARSHIP_LLAMA_SERVER_SHA256`; when unset, the digest is not
    /// checked and the runtime is still used.
    pub fn verify_runtime_hash(path: &Path) -> Result<(), CoreError> {
        let Ok(expected) = std::env::var("STARSHIP_LLAMA_SERVER_SHA256") else {
            return Ok(());
        };
        let bytes = std::fs::read(path).map_err(|e| {
            CoreError::infer(
                "infer.runtime_missing",
                false,
                format!("Cannot read llama-server binary: {e}"),
                None,
            )
        })?;
        let digest = hex::encode(sha2::Sha256::digest(&bytes));
        if digest != expected.trim().to_ascii_lowercase() {
            return Err(CoreError::infer(
                "infer.runtime_missing",
                false,
                "llama-server SHA-256 does not match the configured pin",
                Some(json!({ "expected": expected, "actual": digest })),
            ));
        }
        Ok(())
    }

    /// Spawns the runtime and waits for its health endpoint.
    pub async fn start(
        runtime: &Path,
        model: &Path,
        context_window: usize,
        gpu_layers: u32,
    ) -> Result<Self, CoreError> {
        Self::verify_runtime_hash(runtime)?;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| {
            CoreError::infer(
                "infer.model_load_failed",
                true,
                format!("Cannot allocate a loopback port: {e}"),
                None,
            )
        })?;
        let port = listener
            .local_addr()
            .map_err(|e| {
                CoreError::infer(
                    "infer.model_load_failed",
                    true,
                    format!("Cannot read loopback port: {e}"),
                    None,
                )
            })?
            .port();
        drop(listener);

        let child = tokio::process::Command::new(runtime)
            .arg("-m")
            .arg(model)
            .arg("-c")
            .arg(context_window.to_string())
            .arg("-ngl")
            .arg(gpu_layers.to_string())
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                CoreError::infer(
                    "infer.model_load_failed",
                    false,
                    format!("Failed to start llama-server: {e}"),
                    Some(json!({ "runtime": runtime.display().to_string() })),
                )
            })?;

        let sidecar = Self {
            child: tokio::sync::Mutex::new(Some(child)),
            port,
        };
        sidecar.wait_for_health().await?;
        Ok(sidecar)
    }

    async fn wait_for_health(&self) -> Result<(), CoreError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| {
                CoreError::infer(
                    "infer.model_load_failed",
                    false,
                    format!("Cannot build health client: {e}"),
                    None,
                )
            })?;
        let url = format!("http://127.0.0.1:{}/health", self.port);
        let started = Instant::now();
        while started.elapsed() < STARTUP_TIMEOUT {
            if self.is_dead().await {
                break;
            }
            if client.get(&url).send().await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Err(CoreError::infer(
            "infer.model_load_failed",
            false,
            "llama-server did not become healthy within 30 s",
            Some(json!({ "port": self.port })),
        ))
    }

    async fn is_dead(&self) -> bool {
        let mut guard = self.child.lock().await;
        match guard.as_mut() {
            Some(child) => child.try_wait().ok().flatten().is_some(),
            None => true,
        }
    }

    /// Kills the runtime; safe to call repeatedly.
    pub async fn stop(&self) {
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut() {
            let _ = child.kill().await;
        }
        *guard = None;
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for LlamaSidecar {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.try_lock()
            && let Some(child) = guard.as_mut()
        {
            let _ = child.start_kill();
        }
    }
}
