use std::{any::Any, pin::Pin, sync::Arc};

use derive_builder::Builder;

use crate::{
    Error,
    api_key::ApiKeyRepository,
    artifact::ArtifactRepository,
    issue::IssueRepository,
    project::{ProjectMemberRepository, ProjectRepository},
    user::UserRepository,
};

#[derive(Builder)]
#[builder(pattern = "owned")]
pub struct RepositoryService {
    repository: Arc<dyn Repository>,
    user_repository: Arc<dyn UserRepository>,
    api_key_repository: Arc<dyn ApiKeyRepository>,
    project_repository: Arc<dyn ProjectRepository>,
    project_member_repository: Arc<dyn ProjectMemberRepository>,
    issue_repository: Arc<dyn IssueRepository>,
    artifact_repository: Arc<dyn ArtifactRepository>,
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

    /// Returns a reference to the api key repository.
    #[must_use]
    pub fn api_key_repository(&self) -> &Arc<dyn ApiKeyRepository> {
        &self.api_key_repository
    }

    /// Returns a reference to the project repository.
    #[must_use]
    pub fn project_repository(&self) -> &Arc<dyn ProjectRepository> {
        &self.project_repository
    }

    /// Returns a reference to the project member repository.
    #[must_use]
    pub fn project_member_repository(&self) -> &Arc<dyn ProjectMemberRepository> {
        &self.project_member_repository
    }

    /// Returns a reference to the issue repository.
    #[must_use]
    pub fn issue_repository(&self) -> &Arc<dyn IssueRepository> {
        &self.issue_repository
    }

    /// Returns a reference to the artifact repository.
    #[must_use]
    pub fn artifact_repository(&self) -> &Arc<dyn ArtifactRepository> {
        &self.artifact_repository
    }
}

#[async_trait::async_trait]
pub trait Transaction: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    async fn commit(self: Box<Self>) -> Result<(), Error>;
    async fn rollback(self: Box<Self>) -> Result<(), Error>;
}

#[async_trait::async_trait]
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
#[allow(unused_lifetimes)]
pub trait Repository: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn Transaction>, Error>;
    async fn begin_read_only(&self) -> Result<Box<dyn Transaction>, Error>;
    async fn close(&self) -> Result<(), Error>;
    /// Verify the DB connection is alive.
    async fn ping(&self) -> Result<(), Error>;
}

/// Execute `f` within a read-write transaction, committing on success and
/// rolling back on error.
pub async fn transaction<F, T>(repository: &dyn Repository, f: F) -> Result<T, Error>
where
    F: for<'c> FnOnce(&'c dyn Transaction) -> Pin<Box<dyn std::future::Future<Output = Result<T, Error>> + Send + 'c>> + Send,
    T: Send,
{
    let tx = repository.begin().await?;
    match f(&*tx).await {
        Ok(result) => {
            tx.commit().await?;
            Ok(result)
        }
        Err(e) => {
            let _ = tx.rollback().await;
            Err(e)
        }
    }
}

/// Execute `f` within a read-only transaction, rolling back when done.
pub async fn read_only_transaction<F, T>(repository: &dyn Repository, f: F) -> Result<T, Error>
where
    F: for<'c> FnOnce(&'c dyn Transaction) -> Pin<Box<dyn std::future::Future<Output = Result<T, Error>> + Send + 'c>> + Send,
    T: Send,
{
    let tx = repository.begin_read_only().await?;
    let result = f(&*tx).await;
    let _ = tx.rollback().await;
    result
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
///     user_repository.create(tx, new_user).await
/// })
///
/// // Multiple repositories
/// with_transaction!(self, user_repository, other_repository, |tx| {
///     let user = user_repository.create(tx, new_user).await?;
///     other_repository.do_something(tx, user.id).await
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
/// with_read_only_transaction!(self, user_repository, |tx| {
///     user_repository.find_by_id(tx, id).await
/// })
/// ```
#[macro_export]
macro_rules! with_read_only_transaction {
    ($self:expr, $($repo:ident),+ , |$tx:ident| $body:expr) => {{
        $(let $repo = $self.repository_service.$repo().clone();)+
        $crate::repository::read_only_transaction(&**$self.repository_service.repository(), |$tx| Box::pin(async move { $body })).await
    }};
}

/// Shared test helpers — a no-op `MockTransaction` and a pre-wired
/// `MockRepository` for service unit tests.
#[cfg(any(test, feature = "test-support"))]
pub mod testing {
    use std::{any::Any, sync::Arc};

    use super::{ArtifactRepository, MockRepository, RepositoryServiceBuilder, Transaction};
    use crate::{
        Error,
        api_key::repository::MockApiKeyRepository,
        artifact::MockArtifactRepository,
        issue::repository::MockIssueRepository,
        project::repository::{MockProjectMemberRepository, MockProjectRepository},
        user::repository::MockUserRepository,
    };

    /// A no-op transaction for unit tests.
    pub struct MockTransaction;

    #[async_trait::async_trait]
    impl Transaction for MockTransaction {
        fn as_any(&self) -> &dyn Any {
            self
        }

        async fn commit(self: Box<Self>) -> Result<(), Error> {
            Ok(())
        }

        async fn rollback(self: Box<Self>) -> Result<(), Error> {
            Ok(())
        }
    }

    /// Returns a `MockRepository` pre-configured to open no-op transactions.
    pub fn make_mock_repo() -> MockRepository {
        let mut r = MockRepository::new();
        r.expect_begin()
            .returning(|| Box::pin(async { Ok(Box::new(MockTransaction) as Box<dyn Transaction>) }));
        r.expect_begin_read_only()
            .returning(|| Box::pin(async { Ok(Box::new(MockTransaction) as Box<dyn Transaction>) }));
        r
    }

    /// Returns a `RepositoryServiceBuilder` pre-populated with default mocks.
    /// Override individual repositories for the one(s) under test.
    pub fn default_repository_service_builder() -> RepositoryServiceBuilder {
        RepositoryServiceBuilder::default()
            .repository(Arc::new(make_mock_repo()))
            .user_repository(Arc::new(MockUserRepository::new()))
            .api_key_repository(Arc::new(MockApiKeyRepository::new()))
            .project_repository(Arc::new(MockProjectRepository::new()))
            .project_member_repository(Arc::new(MockProjectMemberRepository::new()))
            .issue_repository(Arc::new(MockIssueRepository::new()))
            .artifact_repository(Arc::new(MockArtifactRepository::new()) as Arc<dyn ArtifactRepository>)
    }
}
