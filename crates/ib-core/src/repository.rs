use std::future::Future;

use crate::error::RepositoryError;

/// A database connection or transaction handle.
///
/// Implemented by `ib-database`; `ib-core` never depends on SeaORM directly.
pub trait Repository: Send + Sync {
    type Transaction<'a>: Transaction
    where
        Self: 'a;

    fn begin_transaction(&self) -> impl Future<Output = Result<Self::Transaction<'_>, RepositoryError>> + Send;
}

/// An active database transaction.
pub trait Transaction: Send + Sync {
    fn commit(self) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn rollback(self) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}
