use super::harness::ChaosMission;
use starship_core::MissionState;
use starship_core::executor::{FailureClass, RetryPolicy, classify_error};
use starship_infer::InferDispatcher;
use starship_infer::types::{ChatRequest, Message};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn fake_status_endpoint(status_line: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind fake API");
    let address = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept request");
        let mut buffer = [0_u8; 4096];
        let _ = stream.read(&mut buffer).await.expect("read request");
        let response =
            format!("HTTP/1.1 {status_line}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    });
    format!("http://{address}/v1")
}

fn chat_request() -> ChatRequest {
    ChatRequest {
        model: "chaos-model".to_owned(),
        messages: vec![Message::user("hello")],
        system_prompt: None,
        temperature: None,
        max_tokens: None,
        tools: Vec::new(),
        grammar: None,
        stream: true,
    }
}

async fn assert_status_is_retryable(status_line: &'static str, code: u16) {
    let base_url = fake_status_endpoint(status_line).await;
    let engine = InferDispatcher::resolve_engine(
        "cloud_openai",
        "chaos-model",
        Some(&base_url),
        Some("test-key"),
        None,
        None,
    )
    .expect("resolve cloud engine");

    let error = engine
        .complete_chat(chat_request())
        .await
        .expect_err("non-2xx response must fail");
    assert!(error.retryable, "{status_line} must be retryable");
    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|details| details.get("status"))
            .and_then(serde_json::Value::as_u64),
        Some(u64::from(code))
    );
    // Current implementation maps both 429 and 5xx to the auth code; recorded as a gap in evidence.
    assert_eq!(error.code, "infer.cloud_auth_failed");
    assert_eq!(classify_error(&error.code), FailureClass::Transient);
    let retry = RetryPolicy::default().evaluate_retry(
        starship_core::safety::RiskTier::Safe,
        FailureClass::Transient,
        true,
        true,
        1,
    );
    assert!(retry.should_retry);

    let chaos = ChaosMission::new();
    chaos.start_step();
    chaos.fail_step(&error.code);
    chaos.assert_final(MissionState::Failed, Some(&error.code));
}

#[tokio::test]
async fn cloud_429_is_retryable() {
    assert_status_is_retryable("429 Too Many Requests", 429).await;
}

#[tokio::test]
async fn cloud_500_is_retryable() {
    assert_status_is_retryable("500 Internal Server Error", 500).await;
}
