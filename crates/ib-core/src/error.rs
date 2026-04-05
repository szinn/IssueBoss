use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Error)]
#[error("{kind}")]
pub struct Error {
    pub kind: ErrorKind,
}

impl Error {
    pub fn not_found() -> Self {
        Self { kind: ErrorKind::NotFound }
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Conflict(msg.into()),
        }
    }

    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidInput(msg.into()),
        }
    }

    pub fn unauthorized() -> Self {
        Self { kind: ErrorKind::Unauthorized }
    }

    pub fn forbidden() -> Self {
        Self { kind: ErrorKind::Forbidden }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Internal(msg.into()),
        }
    }
}

/// Error returned by repository implementations.
#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("database error: {0}")]
    Database(String),
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
}
