use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use orbit_domain::{DomainError, ValidationIssue};
use orbit_security::SecurityError;
use orbit_store::StoreError;

use crate::request_id;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Vec<ValidationIssue>,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorEnvelope {
    pub error: ErrorBody,
}

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Vec<ValidationIssue>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: Vec::new(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    pub fn validation(issues: Vec<ValidationIssue>) -> Self {
        let message = issues
            .first()
            .map(|issue| format!("{}: {}", issue.path, issue.message))
            .unwrap_or_else(|| "validation failed".to_string());
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "validation_error",
            message,
            details: issues,
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "unauthorized", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "conflict", message)
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_IMPLEMENTED, "unsupported", message)
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, "rate_limited", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message)
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if self.status.is_server_error() {
            tracing::error!(code = self.code, message = %self.message, "request failed");
        } else {
            tracing::debug!(code = self.code, message = %self.message, "request rejected");
        }
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code.to_string(),
                message: self.message,
                details: self.details,
                request_id: request_id::current(),
            },
        };
        (self.status, Json(body)).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotFound(what) => ApiError::not_found(what),
            StoreError::Conflict(message) => ApiError::conflict(message),
            StoreError::Invalid(message) => ApiError::bad_request(message),
            other => {
                tracing::error!(error = %other, "store error");
                ApiError::internal("a storage error occurred")
            }
        }
    }
}

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match err {
            DomainError::Validation(report) => ApiError::validation(report.issues),
            DomainError::NotFound(what) => ApiError::not_found(what),
            DomainError::Conflict(message) => ApiError::conflict(message),
            DomainError::Forbidden(message) => ApiError::forbidden(message),
            DomainError::InvalidTransition { from, to } => {
                ApiError::conflict(format!("invalid transition {from} -> {to}"))
            }
            DomainError::Invalid(message) => ApiError::bad_request(message),
            DomainError::Internal(message) => {
                tracing::error!(error = %message, "domain error");
                ApiError::internal("an internal error occurred")
            }
        }
    }
}

impl From<SecurityError> for ApiError {
    fn from(err: SecurityError) -> Self {
        match err {
            SecurityError::Unauthorized => ApiError::unauthorized("authentication required"),
            SecurityError::Forbidden(message) => ApiError::forbidden(message),
            SecurityError::Unavailable(message) => {
                ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "unavailable", message)
            }
            other => {
                tracing::error!(error = %other, "security error");
                ApiError::internal("a security error occurred")
            }
        }
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::bad_request(format!("invalid JSON body: {err}"))
    }
}

impl From<axum::extract::rejection::JsonRejection> for ApiError {
    fn from(err: axum::extract::rejection::JsonRejection) -> Self {
        // Malformed JSON is 400; structurally valid JSON that fails typed
        // deserialization is a validation error (422).
        let code = if err.status() == StatusCode::UNPROCESSABLE_ENTITY {
            "validation_error"
        } else {
            "bad_request"
        };
        ApiError::new(err.status(), code, err.body_text())
    }
}
