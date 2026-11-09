use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("secret error: {0}")]
    Secret(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("dependency unavailable: {0}")]
    Unavailable(String),
    #[error("internal security error: {0}")]
    Internal(String),
}

impl SecurityError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Config(_) | Self::Secret(_) | Self::Internal(_) => "internal",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden(_) => "forbidden",
            Self::Unavailable(_) => "unavailable",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            Self::Unauthorized => 401,
            Self::Forbidden(_) => 403,
            Self::Unavailable(_) => 503,
            _ => 500,
        }
    }
}

/// Safe, redacted representation of a credential presence check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub present: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}
