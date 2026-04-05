use std::{any::Any, sync::Arc};

use derive_builder::Builder;

use crate::{Error, user::UserRepository};

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct RepositoryService {
    repository: Arc<dyn Repository>,
    user_repository: Arc<dyn UserRepository>,
}

impl RepositoryService {
    /// Returns a reference to the main repository for transaction management.
    #[must_use]
    pub fn repository(&self) -> &Arc<dyn Repository> {
        &self.repository
    }

    /// Returns a reference to the user repository.
    #[must_use]
    pub fn user_repository(&self) -> &Arc<dyn UserRepository> {
        &self.user_repository
    }
}

/// A database connection or transaction handle.
///
/// Implemented by `ib-database`; `ib-core` never depends on SeaORM directly.
#[async_trait::async_trait]
pub trait Repository: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn Transaction>, Error>;
    async fn begin_read_only(&self) -> Result<Box<dyn Transaction>, Error>;
    async fn close(&self) -> Result<(), Error>;
    /// Verify the DB connection is alive. Used by `ResilienceWrapper::check()`.
    async fn ping(&self) -> Result<(), Error>;
}

#[async_trait::async_trait]
pub trait Transaction: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    async fn commit(self: Box<Self>) -> Result<(), Error>;
    async fn rollback(self: Box<Self>) -> Result<(), Error>;
}

/// Execute an async operation within a read-write transaction.
///
/// Clones one or more repositories, begins a transaction, executes the body,
/// and commits on success or rolls back on error.
///
/// # Examples
/// ```ignore
/// // Single repository
/// with_transaction!(self, user_repository, |tx| {
///     user_repository.add_user(tx, user).await
/// })
///
/// // Multiple repositories
/// with_transaction!(self, user_repository, order_repository, |tx| {
///     let user = user_repository.add_user(tx, user).await?;
///     order_repository.create_order(tx, user.id, order).await
/// })
/// ```
#[macro_export]
macro_rules! with_transaction {
    ($self:expr, $($repo:ident),+ , |$tx:ident| $body:expr) => {{
        $(let $repo = $self.repository_service.$repo().clone();)+
        $crate::repository::transaction(&**$self.repository_service.repository(), |$tx| Box::pin(async move { $body })).await
    }};
}

/// Execute an async operation within a read-only transaction.
///
/// Clones one or more repositories and executes the body within a read-only
/// transaction.
///
/// # Examples
/// ```ignore
/// // Single repository
/// with_read_only_transaction!(self, user_repository, |tx| {
///     user_repository.find_by_id(tx, id).await
/// })
///
/// // Multiple repositories
/// with_read_only_transaction!(self, user_repository, order_repository, |tx| {
///     let user = user_repository.find_by_id(tx, id).await?;
///     let orders = order_repository.find_by_user(tx, user.id).await?;
///     Ok((user, orders))
/// })
/// ```
#[macro_export]
macro_rules! with_read_only_transaction {
    ($self:expr, $($repo:ident),+ , |$tx:ident| $body:expr) => {{
        $(let $repo = $self.repository_service.$repo().clone();)+
        $crate::repository::read_only_transaction(&**$self.repository_service.repository(), |$tx| Box::pin(async move { $body })).await
    }};
}
