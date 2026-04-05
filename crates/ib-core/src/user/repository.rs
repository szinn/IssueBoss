use std::future::Future;

use super::model::{User, UserId};
use crate::error::RepositoryError;

pub trait UserRepository: Send + Sync {
    fn find_by_id(&self, id: UserId) -> impl Future<Output = Result<Option<User>, RepositoryError>> + Send;

    fn find_by_username(&self, username: &str) -> impl Future<Output = Result<Option<User>, RepositoryError>> + Send;

    fn find_by_api_key_hash(&self, hash: &str) -> impl Future<Output = Result<Option<User>, RepositoryError>> + Send;

    fn create(&self, user: User) -> impl Future<Output = Result<User, RepositoryError>> + Send;

    fn update(&self, user: User) -> impl Future<Output = Result<User, RepositoryError>> + Send;

    fn delete(&self, id: UserId) -> impl Future<Output = Result<(), RepositoryError>> + Send;

    fn any_super_admin(&self) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}
