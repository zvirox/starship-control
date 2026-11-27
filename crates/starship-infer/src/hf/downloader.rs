use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use starship_core::CoreError;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    #[serde(rename = "downloadJobId")]
    pub download_job_id: String,
    #[serde(rename = "downloadedBytes")]
    pub downloaded_bytes: u64,
    #[serde(rename = "totalBytes")]
    pub total_bytes: u64,
    #[serde(rename = "speedBytesPerSec")]
    pub speed_bytes_per_sec: u64,
    #[serde(rename = "etaSeconds")]
    pub eta_seconds: u64,
    #[serde(rename = "progressFraction")]
    pub progress_fraction: f32,
}

/// Downloads a GGUF model file with chunked resumable transfers, progress streaming, and SHA-256 integrity verification.
/// Rejects files that are not real GGUF weights before registering them.
/// A truncated transfer or an HTML error page must never become a "ready" model.
fn verify_gguf_file(path: &Path, expected_size: u64) -> Result<(), CoreError> {
    use std::io::Read;

    let mut file = fs::File::open(path).map_err(|e| {
        CoreError::infer(
            "infer.checksum_mismatch",
            false,
            format!("Cannot verify downloaded model: {e}"),
            None,
        )
    })?;
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic).map_err(|_| {
        CoreError::infer(
            "infer.checksum_mismatch",
            false,
            "Downloaded file is too small to be a GGUF model",
            None,
        )
    })?;
    if &magic != b"GGUF" {
        return Err(CoreError::infer(
            "infer.checksum_mismatch",
            false,
            "Downloaded file is not a GGUF model (bad magic bytes)",
            Some(serde_json::json!({ "magic": hex::encode(magic) })),
        ));
    }
    let size = fs::metadata(path).map_or(0, |m| m.len());
    if expected_size > 0 && size != expected_size {
        return Err(CoreError::infer(
            "infer.checksum_mismatch",
            false,
            format!("Downloaded file size {size} does not match expected {expected_size}"),
            None,
        ));
    }
    Ok(())
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub async fn download_hf_file<F>(
    job_id: &str,
    url: &str,
    target_path: &Path,
    expected_sha256: Option<&str>,
    on_progress: F,
) -> Result<String, CoreError>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            CoreError::infer(
                "infer.fs_error",
                false,
                format!("Failed to create model directory: {e}"),
                None,
            )
        })?;
    }

    let part_path = PathBuf::from(format!("{}.part", target_path.display()));
    let mut initial_downloaded = 0u64;
    let mut hasher = Sha256::new();

    // Check if partial download exists to support resumption
    if let Ok(meta) = fs::metadata(&part_path) {
        initial_downloaded = meta.len();
        // Pre-hash existing bytes
        if let Ok(mut f) = fs::File::open(&part_path) {
            let mut buf = [0u8; 8192];
            while let Ok(n) = f.read(&mut buf) {
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
        }
    }

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();

    if initial_downloaded > 0 {
        let range_header = format!("bytes={initial_downloaded}-");
        if let Ok(val) = HeaderValue::from_str(&range_header) {
            headers.insert(RANGE, val);
        }
    }

    let res = client.get(url).headers(headers).send().await.map_err(|e| {
        CoreError::infer(
            "infer.download_failed",
            true,
            format!("Failed to connect to Hugging Face CDN at {url}: {e}"),
            None,
        )
    })?;

    let status = res.status();
    if !status.is_success() && status.as_u16() != 206 {
        return Err(CoreError::infer(
            "infer.download_failed",
            status.is_server_error() || status.as_u16() == 429,
            format!("Hugging Face CDN returned error status {status}"),
            Some(serde_json::json!({ "status": status.as_u16() })),
        ));
    }

    let total_bytes = if status.as_u16() == 206 {
        // Content-Range or Content-Length + initial
        res.content_length().unwrap_or(0) + initial_downloaded
    } else {
        // Starting fresh
        initial_downloaded = 0;
        hasher = Sha256::new();
        res.content_length().unwrap_or(0)
    };

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(initial_downloaded > 0)
        .truncate(initial_downloaded == 0)
        .open(&part_path)
        .map_err(|e| {
            CoreError::infer(
                "infer.fs_error",
                false,
                format!(
                    "Failed to open destination file {}: {e}",
                    part_path.display()
                ),
                None,
            )
        })?;

    let mut stream = res.bytes_stream();
    let mut downloaded = initial_downloaded;
    let start_time = Instant::now();
    let mut last_progress_time = Instant::now();

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.map_err(|e| {
            CoreError::infer(
                "infer.download_failed",
                true,
                format!("Connection interrupted during chunk download: {e}"),
                None,
            )
        })?;

        file.write_all(&chunk).map_err(|e| {
            CoreError::infer(
                "infer.fs_error",
                false,
                format!("Failed to write chunk to disk: {e}"),
                None,
            )
        })?;

        hasher.update(&chunk);
        downloaded += chunk.len() as u64;

        // Emit progress at ~4 Hz (every 250ms)
        if last_progress_time.elapsed().as_millis() >= 250 || downloaded == total_bytes {
            let elapsed_secs = start_time.elapsed().as_secs_f64().max(0.001);
            let bytes_in_session = downloaded.saturating_sub(initial_downloaded);
            let speed = (bytes_in_session as f64 / elapsed_secs) as u64;

            let remaining_bytes = total_bytes.saturating_sub(downloaded);
            let eta = remaining_bytes.checked_div(speed).unwrap_or(0);

            let progress_fraction = if total_bytes > 0 {
                (downloaded as f32 / total_bytes as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };

            on_progress(DownloadProgress {
                download_job_id: job_id.to_string(),
                downloaded_bytes: downloaded,
                total_bytes,
                speed_bytes_per_sec: speed,
                eta_seconds: eta,
                progress_fraction,
            });

            last_progress_time = Instant::now();
        }
    }

    file.flush().map_err(|e| {
        CoreError::infer(
            "infer.fs_error",
            false,
            format!("Failed to flush file to disk: {e}"),
            None,
        )
    })?;

    let final_sha = hex::encode(hasher.finalize());

    // Verify SHA-256 checksum if provided
    if let Some(expected) = expected_sha256
        && !final_sha.eq_ignore_ascii_case(expected)
    {
        let _ = fs::remove_file(&part_path);
        return Err(CoreError::infer(
            "infer.checksum_mismatch",
            false,
            format!("SHA-256 integrity check failed. Expected: {expected}, Computed: {final_sha}"),
            Some(serde_json::json!({
                "expected": expected,
                "computed": final_sha,
            })),
        ));
    }

    // Verify the actual bytes before promoting the file to a usable model.
    verify_gguf_file(&part_path, total_bytes)?;

    // Rename .part to final verified model file
    fs::rename(&part_path, target_path).map_err(|e| {
        CoreError::infer(
            "infer.fs_error",
            false,
            format!("Failed to finalize model file name: {e}"),
            None,
        )
    })?;

    Ok(final_sha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn serve(bytes: &[u8], partial: bool) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind stub");
        let port = listener.local_addr().expect("port").port();
        let payload = bytes.to_vec();
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buffer = [0_u8; 4096];
            let read = socket.read(&mut buffer).await.expect("read request");
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let body = if partial && request.contains("Range: bytes=") {
                let start: usize = request
                    .split("Range: bytes=")
                    .nth(1)
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.split('-').next())
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
                payload[start..].to_vec()
            } else {
                payload.clone()
            };
            let status = if partial && request.contains("Range: bytes=") {
                "206 Partial Content"
            } else {
                "200 OK"
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("headers");
            socket.write_all(&body).await.expect("body");
        });
        (format!("http://127.0.0.1:{port}/model.gguf"), handle)
    }

    #[tokio::test]
    async fn test_download_registers_only_real_gguf_bytes() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("tiny.gguf");
        let payload = b"GGUF\x00\x00\x00tiny-weights";
        let (url, server) = serve(payload, false).await;
        let sha = download_hf_file("job", &url, &target, None, |_| {})
            .await
            .expect("valid GGUF download");
        server.await.expect("stub");
        assert_eq!(sha, hex::encode(Sha256::digest(payload)));
        assert_eq!(fs::read(&target).expect("file"), payload.to_vec());
        assert!(verify_gguf_file(&target, payload.len() as u64).is_ok());
    }

    #[tokio::test]
    async fn test_corrupt_download_is_rejected_without_promoting_file() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("bad.gguf");
        let (url, server) = serve(b"<html>not a model</html>", false).await;
        let err = download_hf_file("job", &url, &target, None, |_| {})
            .await
            .expect_err("corrupt file must fail");
        server.await.expect("stub");
        assert_eq!(err.code, "infer.checksum_mismatch");
        assert!(!target.exists(), "partial file must stay unpromoted");
    }

    #[tokio::test]
    async fn test_partial_download_resumes_from_range() {
        let dir = tempdir().expect("tempdir");
        let target = dir.path().join("resume.gguf");
        let payload = b"GGUF\x03\x00\x00\x00resumed-weights";
        fs::write(format!("{}.part", target.display()), &payload[..8]).expect("seed part");
        let (url, server) = serve(payload, true).await;
        let sha = download_hf_file("job", &url, &target, None, |_| {})
            .await
            .expect("resumed download");
        server.await.expect("stub");
        assert_eq!(sha, hex::encode(Sha256::digest(payload)));
        assert_eq!(fs::read(&target).expect("file"), payload.to_vec());
    }
}
