use serde::Serialize;
use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub struct HostError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl HostError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(code: &str, message: impl Into<String>, details: Value) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: Some(details),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for HostError {}

pub type HostResult<T> = Result<T, HostError>;
