use std::sync::Arc;

pub mod error;

pub use error::*;

mod adapters;
mod entities;
mod migrations;
mod repository;
mod transaction;

use ib_core::{
    Error,
    api_key::ApiKeyRepository,
    artifact::ArtifactRepository,
    issue::IssueRepository,
    project::{ProjectMemberRepository, ProjectRepository},
    repository::{Repository, RepositoryService, RepositoryServiceBuilder},
    user::UserRepository,
};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::{
    adapters::{
        ApiKeyRepositoryAdapter, ArtifactRepositoryAdapter, IssueRepositoryAdapter, ProjectMemberRepositoryAdapter, ProjectRepositoryAdapter,
        UserRepositoryAdapter,
    },
    migrations::Migrator,
    repository::RepositoryImpl,
};

pub async fn open_database(database_url: &str) -> Result<DatabaseConnection, Error> {
    tracing::debug!("Connecting to database...");
    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(9)
        .min_connections(5)
        .sqlx_logging(true)
        .sqlx_logging_level(tracing::log::LevelFilter::Info);

    Ok(Database::connect(opt).await.map_err(handle_dberr)?)
}

pub async fn create_repository_service(database: DatabaseConnection) -> Result<Arc<RepositoryService>, Error> {
    let span = tracing::span!(tracing::Level::TRACE, "Migrations").entered();
    Migrator::up(&database, None).await.map_err(handle_dberr)?;
    span.exit();

    let repository_service = RepositoryServiceBuilder::default()
        .repository(Arc::new(RepositoryImpl::new(database)) as Arc<dyn Repository>)
        .user_repository(Arc::new(UserRepositoryAdapter::new()) as Arc<dyn UserRepository>)
        .api_key_repository(Arc::new(ApiKeyRepositoryAdapter::new()) as Arc<dyn ApiKeyRepository>)
        .project_repository(Arc::new(ProjectRepositoryAdapter::new()) as Arc<dyn ProjectRepository>)
        .project_member_repository(Arc::new(ProjectMemberRepositoryAdapter::new()) as Arc<dyn ProjectMemberRepository>)
        .issue_repository(Arc::new(IssueRepositoryAdapter::new()) as Arc<dyn IssueRepository>)
        .artifact_repository(Arc::new(ArtifactRepositoryAdapter::new()) as Arc<dyn ArtifactRepository>)
        .build()
        .map_err(|e| Error::Infrastructure(e.to_string()))?;

    Ok(Arc::new(repository_service))
}
