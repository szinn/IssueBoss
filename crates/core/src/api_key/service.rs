use std::sync::Arc;

use chrono::{TimeDelta, Utc};

use crate::{
    Error, RepositoryError,
    api_key::model::{ApiKey, ApiKeyId, NewApiKey, generate_api_key},
    repository::RepositoryService,
    user::UserId,
    with_read_only_transaction, with_transaction,
};

/// Only write `last_used_at` if it is `None` or older than this threshold.
/// This avoids a DB write on every request within a single MCP session.
const LAST_USED_THRESHOLD: TimeDelta = TimeDelta::hours(24);

#[async_trait::async_trait]
pub trait ApiKeyService: Send + Sync {
    /// Create a new API key for a user. Returns the stored record and the
    /// plaintext key (shown once — caller must deliver it to the user).
    async fn create_key(&self, new_key: NewApiKey) -> Result<(ApiKey, String), Error>;

    /// Revoke a single key by its ID.
    async fn revoke_key(&self, id: ApiKeyId) -> Result<ApiKey, Error>;

    /// Revoke all keys belonging to a user (e.g. on account deletion).
    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<Vec<ApiKey>, Error>;

    /// Look up a key by its SHA-256 hash — hot path for every authenticated
    /// request.
    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, Error>;

    /// List all active keys for a user (for the "manage API keys" UI).
    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<ApiKey>, Error>;

    /// Record that a key was just used (updates last_used_at).
    /// Skips the write if `last_used_at` is already within
    /// [`LAST_USED_THRESHOLD`]. Fire-and-forget in auth middleware —
    /// failure should not block the request.
    async fn record_usage(&self, api_key: ApiKey) -> Result<(), Error>;
}

pub(crate) struct ApiKeyServiceImpl {
    repository_service: Arc<RepositoryService>,
}

impl ApiKeyServiceImpl {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        Self { repository_service }
    }
}

#[async_trait::async_trait]
impl ApiKeyService for ApiKeyServiceImpl {
    async fn create_key(&self, new_key: NewApiKey) -> Result<(ApiKey, String), Error> {
        let key_type = new_key.key_type.clone();
        with_transaction!(self, api_key_repository, |tx| {
            let generated = generate_api_key(&key_type);
            let key = api_key_repository.create(tx, new_key, generated.hash, generated.prefix).await?;
            Ok((key, generated.plaintext))
        })
    }

    async fn revoke_key(&self, id: ApiKeyId) -> Result<ApiKey, Error> {
        with_transaction!(self, api_key_repository, |tx| {
            api_key_repository
                .find_by_id(tx, id)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            api_key_repository.delete(tx, id).await
        })
    }

    async fn revoke_all_for_user(&self, user_id: UserId) -> Result<Vec<ApiKey>, Error> {
        with_transaction!(self, api_key_repository, |tx| api_key_repository.delete_all_for_user(tx, user_id).await)
    }

    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKey>, Error> {
        let hash = hash.to_owned();
        with_read_only_transaction!(self, api_key_repository, |tx| api_key_repository.find_by_hash(tx, &hash).await)
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<ApiKey>, Error> {
        with_read_only_transaction!(self, api_key_repository, |tx| api_key_repository.list_for_user(tx, user_id).await)
    }

    async fn record_usage(&self, api_key: ApiKey) -> Result<(), Error> {
        let needs_update = match api_key.last_used_at {
            None => true,
            Some(last_used) => Utc::now() - last_used > LAST_USED_THRESHOLD,
        };
        if !needs_update {
            return Ok(());
        }
        let id = api_key.id;
        with_transaction!(self, api_key_repository, |tx| api_key_repository.update_last_used(tx, id).await)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;

    use super::{ApiKeyService, ApiKeyServiceImpl};
    use crate::{
        Error, RepositoryError,
        api_key::{
            model::{ApiKey, NewApiKey},
            repository::MockApiKeyRepository,
        },
        repository::testing::default_repository_service_builder,
    };

    fn make_svc(repo: MockApiKeyRepository) -> ApiKeyServiceImpl {
        let repo_svc = Arc::new(default_repository_service_builder().api_key_repository(Arc::new(repo)).build().unwrap());
        ApiKeyServiceImpl::new(repo_svc)
    }

    fn fake_key(id: u64, user_id: u64, key_type: &str) -> ApiKey {
        ApiKey {
            id,
            user_id,
            key_type: key_type.to_owned(),
            key_hash: "hash".to_owned(),
            key_prefix: "ib_live_XXXX".to_owned(),
            name: None,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    fn new_key(user_id: u64) -> NewApiKey {
        NewApiKey {
            user_id,
            key_type: "ib_live".to_owned(),
            name: None,
        }
    }

    #[tokio::test]
    async fn create_key_returns_key_and_plaintext() {
        let key = fake_key(1, 10, "ib_live");
        let mut repo = MockApiKeyRepository::new();
        repo.expect_create().returning(move |_, _, _, _| {
            let k = key.clone();
            Box::pin(async move { Ok(k) })
        });
        let (returned_key, plaintext) = make_svc(repo).create_key(new_key(10)).await.unwrap();
        assert_eq!(returned_key.user_id, 10);
        assert!(plaintext.starts_with("ib_live_"));
    }

    #[tokio::test]
    async fn revoke_key_not_found_returns_error() {
        let mut repo = MockApiKeyRepository::new();
        repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let result = make_svc(repo).revoke_key(999).await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    #[tokio::test]
    async fn revoke_key_success() {
        let key = fake_key(1, 10, "ib_live");
        let deleted = key.clone();
        let mut repo = MockApiKeyRepository::new();
        repo.expect_find_by_id().returning(move |_, _| {
            let k = key.clone();
            Box::pin(async move { Ok(Some(k)) })
        });
        repo.expect_delete().returning(move |_, _| {
            let k = deleted.clone();
            Box::pin(async move { Ok(k) })
        });
        let result = make_svc(repo).revoke_key(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 1);
    }

    #[tokio::test]
    async fn find_by_hash_delegates_to_repository() {
        let key = fake_key(1, 10, "ib_live");
        let mut repo = MockApiKeyRepository::new();
        repo.expect_find_by_hash().returning(move |_, _| {
            let k = key.clone();
            Box::pin(async move { Ok(Some(k)) })
        });
        let result = make_svc(repo).find_by_hash("somehash").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 1);
    }

    #[tokio::test]
    async fn list_for_user_returns_all_keys() {
        let keys = vec![fake_key(1, 10, "ib_live"), fake_key(2, 10, "ib_live")];
        let expected = keys.clone();
        let mut repo = MockApiKeyRepository::new();
        repo.expect_list_for_user().returning(move |_, _| {
            let k = expected.clone();
            Box::pin(async move { Ok(k) })
        });
        let result = make_svc(repo).list_for_user(10).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn revoke_all_for_user_delegates_to_repository() {
        let keys = vec![fake_key(1, 10, "ib_live")];
        let expected = keys.clone();
        let mut repo = MockApiKeyRepository::new();
        repo.expect_delete_all_for_user().returning(move |_, _| {
            let k = expected.clone();
            Box::pin(async move { Ok(k) })
        });
        let result = make_svc(repo).revoke_all_for_user(10).await.unwrap();
        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn record_usage_writes_when_last_used_is_none() {
        let key = fake_key(1, 10, "ib_live"); // last_used_at: None
        let mut repo = MockApiKeyRepository::new();
        repo.expect_update_last_used().once().returning(|_, _| Box::pin(async { Ok(()) }));
        make_svc(repo).record_usage(key).await.unwrap();
    }

    #[tokio::test]
    async fn record_usage_skips_write_when_recently_used() {
        let mut key = fake_key(1, 10, "ib_live");
        key.last_used_at = Some(Utc::now() - chrono::TimeDelta::minutes(1));
        let repo = MockApiKeyRepository::new();
        // expect_update_last_used NOT called — mock will panic if it is
        let result = make_svc(repo).record_usage(key).await;
        result.unwrap();
    }

    #[tokio::test]
    async fn record_usage_writes_when_last_used_is_stale() {
        let mut key = fake_key(1, 10, "ib_live");
        key.last_used_at = Some(Utc::now() - chrono::TimeDelta::hours(25));
        let mut repo = MockApiKeyRepository::new();
        repo.expect_update_last_used().once().returning(|_, _| Box::pin(async { Ok(()) }));
        make_svc(repo).record_usage(key).await.unwrap();
    }
}
