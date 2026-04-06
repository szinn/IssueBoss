pub mod api_key;
pub mod error;
pub mod repository;
pub mod user;

use std::sync::Arc;

pub use error::{Error, ErrorKind, RepositoryError};

use crate::{
    api_key::{ApiKeyService, service::ApiKeyServiceImpl},
    repository::RepositoryService,
    user::{UserService, service::UserServiceImpl},
};

pub struct CoreServices {
    user_service: Arc<dyn UserService>,
    api_key_service: Arc<dyn ApiKeyService>,
}

impl CoreServices {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        let user_service = Arc::new(UserServiceImpl::new(repository_service.clone()));
        let api_key_service = Arc::new(ApiKeyServiceImpl::new(repository_service.clone()));
        Self { user_service, api_key_service }
    }

    #[must_use]
    pub fn user_service(&self) -> &Arc<dyn UserService> {
        &self.user_service
    }

    #[must_use]
    pub fn api_key_service(&self) -> &Arc<dyn ApiKeyService> {
        &self.api_key_service
    }
}

pub fn create_services(repository_service: Arc<RepositoryService>) -> Arc<CoreServices> {
    Arc::new(CoreServices::new(repository_service))
}
