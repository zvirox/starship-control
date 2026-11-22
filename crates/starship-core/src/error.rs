use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Standard error families defined in API.md §1.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Infer,
    Mcp,
    Safety,
    Db,
    App,
    Settings,
    Ipc,
    Plan,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Infer => write!(f, "infer"),
            Self::Mcp => write!(f, "mcp"),
            Self::Safety => write!(f, "safety"),
            Self::Db => write!(f, "db"),
            Self::App => write!(f, "app"),
            Self::Settings => write!(f, "settings"),
            Self::Ipc => write!(f, "ipc"),
            Self::Plan => write!(f, "plan"),
        }
    }
}

/// Standard error envelope strictly adhering to API.md §1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Error)]
#[error("[{kind}] {code}: {user_message}")]
pub struct CoreError {
    pub kind: ErrorKind,
    pub code: String,
    pub retryable: bool,
    #[serde(rename = "userMessage")]
    pub user_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl CoreError {
    #[must_use]
    pub fn new(
        kind: ErrorKind,
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            kind,
            code: code.into(),
            retryable,
            user_message: user_message.into(),
            details,
        }
    }

    #[must_use]
    pub fn db(
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::new(ErrorKind::Db, code, retryable, user_message, details)
    }

    #[must_use]
    pub fn ipc(
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::new(ErrorKind::Ipc, code, retryable, user_message, details)
    }

    #[must_use]
    pub fn infer(
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::new(ErrorKind::Infer, code, retryable, user_message, details)
    }

    #[must_use]
    pub fn mcp(
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::new(ErrorKind::Mcp, code, retryable, user_message, details)
    }

    #[must_use]
    pub fn safety(
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::new(ErrorKind::Safety, code, retryable, user_message, details)
    }

    #[must_use]
    pub fn app(
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::new(ErrorKind::App, code, retryable, user_message, details)
    }

    #[must_use]
    pub fn plan(
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::new(ErrorKind::Plan, code, retryable, user_message, details)
    }

    #[must_use]
    pub fn settings(
        code: impl Into<String>,
        retryable: bool,
        user_message: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self::new(ErrorKind::Settings, code, retryable, user_message, details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_core_error_serialization() {
        let err = CoreError::infer(
            "infer.model_out_of_memory",
            false,
            "Local model weights exceed available VRAM.",
            Some(json!({ "backend": "vulkan" })),
        );

        let serialized = serde_json::to_string(&err).expect("serialize");
        assert!(serialized.contains("\"kind\":\"infer\""));
        assert!(serialized.contains("\"code\":\"infer.model_out_of_memory\""));
        assert!(
            serialized.contains("\"userMessage\":\"Local model weights exceed available VRAM.\"")
        );
        assert!(serialized.contains("\"details\":{\"backend\":\"vulkan\"}"));

        let deserialized: CoreError = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(err, deserialized);
    }
}
