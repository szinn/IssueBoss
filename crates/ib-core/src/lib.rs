pub mod error;
pub mod repository;
pub mod user;

use std::sync::Arc;

pub use error::{Error, ErrorKind, RepositoryError};

use crate::repository::RepositoryService;

pub struct CoreServices {
    pub(crate) repository_service: Arc<RepositoryService>,
}

impl CoreServices {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        Self {
            repository_service: repository_service.clone(),
        }
    }
}

pub fn create_services(repository_service: Arc<RepositoryService>) -> Arc<CoreServices> {
    Arc::new(CoreServices::new(repository_service))
}
