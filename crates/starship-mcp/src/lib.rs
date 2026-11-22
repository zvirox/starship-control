use serde::{Deserialize, Serialize};
use starship_core::CoreError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
}

pub struct McpClient {
    pub socket_path: Option<String>,
}

impl McpClient {
    #[must_use]
    pub fn new(socket_path: Option<String>) -> Self {
        Self { socket_path }
    }

    pub async fn ping(&self) -> Result<bool, CoreError> {
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_ping() {
        let client = McpClient::new(None);
        let ok = client.ping().await.expect("ping");
        assert!(ok);
    }
}
