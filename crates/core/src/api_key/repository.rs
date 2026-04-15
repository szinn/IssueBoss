use super::model::{ApiKey, ApiKeyId, NewApiKey};
use crate::{Error, repository::Transaction, user::UserId};

#[async_trait::async_trait]
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
pub trait ApiKeyRepository: Send + Sync {
    async fn create(&self, tx: &dyn Transaction, new_key: NewApiKey, hash: String, prefix: String) -> Result<ApiKey, Error>;

    async fn find_by_id(&self, tx: &dyn Transaction, id: ApiKeyId) -> Result<Option<ApiKey>, Error>;

    async fn find_by_hash(&self, tx: &dyn Transaction, hash: &str) -> Result<Option<ApiKey>, Error>;

    async fn list_for_user(&self, tx: &dyn Transaction, user_id: UserId) -> Result<Vec<ApiKey>, Error>;

    async fn delete(&self, tx: &dyn Transaction, id: ApiKeyId) -> Result<ApiKey, Error>;

    async fn delete_all_for_user(&self, tx: &dyn Transaction, user_id: UserId) -> Result<Vec<ApiKey>, Error>;

    async fn update_last_used(&self, tx: &dyn Transaction, id: ApiKeyId) -> Result<(), Error>;
}
