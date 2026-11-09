use crate::{ExecutionStatus, ValidationReport};

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("validation failed")]
    Validation(ValidationReport),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("invalid execution transition: {from} -> {to}")]
    InvalidTransition {
        from: ExecutionStatus,
        to: ExecutionStatus,
    },
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl DomainError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation_error",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::Forbidden(_) => "forbidden",
            Self::InvalidTransition { .. } | Self::Invalid(_) => "bad_request",
            Self::Internal(_) => "internal",
        }
    }

    pub fn issues(&self) -> Vec<crate::ValidationIssue> {
        match self {
            Self::Validation(report) => report.issues.clone(),
            _ => Vec::new(),
        }
    }
}

pub type DomainResult<T> = Result<T, DomainError>;
