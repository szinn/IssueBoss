#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Resource not found.
    NotFound,
    /// Resource conflict, e.g., optimistic locking failure.
    Conflict,
    /// Invalid input or constraint violation.
    InvalidInput,
    /// Malformed request data.
    BadRequest,
    /// Internal or infrastructure error.
    Internal,
    /// Service temporarily unavailable (transient — DB or storage unreachable).
    ServiceUnavailable,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Invalid ID: {0}")]
    InvalidId(u64),

    #[error("Invalid page size: {0}")]
    InvalidPageSize(u64),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid transaction type")]
    InvalidTransactionType,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Infrastructure error: {0}")]
    Infrastructure(String),

    /// Storage system unreachable (NFS gone, library path missing). Transient.
    #[error("Storage unavailable: {0}")]
    StorageUnavailable(String),

    #[error("Frontend error: {0}")]
    FrontendError(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Gate failure: condition={condition}")]
    GateFailure { condition: String, failing_tokens: Vec<String> },

    #[error("Dependency cycle detected")]
    CycleDetected,

    #[error("Already exists")]
    AlreadyExists,

    #[error(transparent)]
    RepositoryError(#[from] RepositoryError),
}

impl Error {
    /// Returns the error kind for response mapping in adapters.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::InvalidId(_) | Self::InvalidPageSize(_) | Self::InvalidToken(_) => ErrorKind::BadRequest,
            Self::Validation(_) | Self::GateFailure { .. } | Self::CycleDetected => ErrorKind::InvalidInput,
            Self::AlreadyExists => ErrorKind::Conflict,
            Self::InvalidTransactionType | Self::Infrastructure(_) | Self::CryptoError(_) | Self::FrontendError(_) => ErrorKind::Internal,
            Self::StorageUnavailable(_) => ErrorKind::ServiceUnavailable,
            Self::RepositoryError(e) => e.kind(),
        }
    }

    /// Returns `true` for errors caused by transient infrastructure failures
    /// (DB connectivity loss, NFS unavailable). The server can recover without
    /// a restart; subsystems should retry rather than propagating these.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        matches!(self, Self::RepositoryError(RepositoryError::Connection(_)) | Self::StorageUnavailable(_))
    }
}

/// Error returned by repository implementations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum RepositoryError {
    #[error("Constraint Error - {0}")]
    Constraint(String),

    #[error("Conflict Error")]
    Conflict,

    #[error("Not found")]
    NotFound,

    #[error("Read-only Transaction")]
    ReadOnly,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Query canceled")]
    QueryCanceled,

    /// Database connection lost (DNS failure, network partition, pool
    /// exhausted). Transient — callers should retry.
    #[error("Connection Error: {0}")]
    Connection(String),
}

impl RepositoryError {
    /// Returns the error kind for response mapping in adapters.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound => ErrorKind::NotFound,
            Self::Conflict => ErrorKind::Conflict,
            Self::Constraint(_) => ErrorKind::InvalidInput,
            Self::ReadOnly | Self::Database(_) | Self::QueryCanceled => ErrorKind::Internal,
            Self::Connection(_) => ErrorKind::ServiceUnavailable,
        }
    }
}
