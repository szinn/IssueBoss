use super::model::{NewUser, User, UserId};
use crate::{error::Error, repository::Transaction};

#[async_trait::async_trait]
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
pub trait UserRepository: Send + Sync {
    async fn list(&self, transaction: &dyn Transaction) -> Result<Vec<User>, Error>;
    async fn find_by_id(&self, transaction: &dyn Transaction, id: UserId) -> Result<Option<User>, Error>;
    async fn find_by_username(&self, transaction: &dyn Transaction, username: &str) -> Result<Option<User>, Error>;
    async fn find_by_api_key_hash(&self, transaction: &dyn Transaction, hash: &str) -> Result<Option<User>, Error>;
    async fn create(&self, transaction: &dyn Transaction, new_user: NewUser) -> Result<User, Error>;
    async fn update(&self, transaction: &dyn Transaction, user: User) -> Result<User, Error>;
    async fn delete(&self, transaction: &dyn Transaction, user: User) -> Result<User, Error>;
    async fn any_super_admin(&self, transaction: &dyn Transaction) -> Result<bool, Error>;
}
