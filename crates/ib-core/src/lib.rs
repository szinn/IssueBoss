pub mod api_key;
pub mod artifact;
pub mod error;
pub mod issue;
pub mod project;
pub mod repository;
pub mod types;
pub mod user;

use std::sync::Arc;

pub use error::{Error, ErrorKind, RepositoryError};

use crate::{
    api_key::{ApiKeyService, service::ApiKeyServiceImpl},
    artifact::{ArtifactService, service::ArtifactServiceImpl},
    issue::{IssueService, service::IssueServiceImpl},
    project::{ProjectService, service::ProjectServiceImpl},
    repository::RepositoryService,
    user::{UserService, service::UserServiceImpl},
};

pub struct CoreServices {
    user_service: Arc<dyn UserService>,
    api_key_service: Arc<dyn ApiKeyService>,
    project_service: Arc<dyn ProjectService>,
    issue_service: Arc<dyn IssueService>,
    artifact_service: Arc<dyn ArtifactService>,
}

impl CoreServices {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        let user_service = Arc::new(UserServiceImpl::new(repository_service.clone()));
        let api_key_service = Arc::new(ApiKeyServiceImpl::new(repository_service.clone()));
        let project_service = Arc::new(ProjectServiceImpl::new(repository_service.clone()));
        let issue_service = Arc::new(IssueServiceImpl::new(repository_service.clone()));
        let artifact_service = Arc::new(ArtifactServiceImpl::new(repository_service.clone()));
        Self {
            user_service,
            api_key_service,
            project_service,
            issue_service,
            artifact_service,
        }
    }

    #[must_use]
    pub fn user_service(&self) -> &Arc<dyn UserService> {
        &self.user_service
    }

    #[must_use]
    pub fn api_key_service(&self) -> &Arc<dyn ApiKeyService> {
        &self.api_key_service
    }

    #[must_use]
    pub fn project_service(&self) -> &Arc<dyn ProjectService> {
        &self.project_service
    }

    #[must_use]
    pub fn issue_service(&self) -> &Arc<dyn IssueService> {
        &self.issue_service
    }

    #[must_use]
    pub fn artifact_service(&self) -> &Arc<dyn ArtifactService> {
        &self.artifact_service
    }
}

pub fn create_services(repository_service: Arc<RepositoryService>) -> Arc<CoreServices> {
    Arc::new(CoreServices::new(repository_service))
}
