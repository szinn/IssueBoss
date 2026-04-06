pub mod error;
pub mod repository;
pub mod user;

use std::sync::Arc;

pub use error::{Error, ErrorKind, RepositoryError};

use crate::{
    repository::RepositoryService,
    user::{UserService, service::UserServiceImpl},
};

pub struct CoreServices {
    user_service: Arc<dyn UserService>,
}

impl CoreServices {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        let user_service = Arc::new(UserServiceImpl::new(repository_service.clone()));
        Self { user_service }
    }

    #[must_use]
    pub fn user_service(&self) -> &Arc<dyn UserService> {
        &self.user_service
    }
}

pub fn create_services(repository_service: Arc<RepositoryService>) -> Arc<CoreServices> {
    Arc::new(CoreServices::new(repository_service))
}
