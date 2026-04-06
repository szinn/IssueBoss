use chrono::Utc;
use ib_core::{
    Error, RepositoryError,
    api_key::{ApiKey, ApiKeyId, ApiKeyRepository, NewApiKey},
    repository::Transaction,
    user::UserId,
};
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, QueryFilter, Set};

use crate::{
    entities::{
        api_keys::{self, Entity as ApiKeyEntity},
        prelude,
    },
    handle_dberr,
    transaction::TransactionImpl,
};

pub(crate) struct ApiKeyRepositoryAdapter;

impl ApiKeyRepositoryAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl From<api_keys::Model> for ApiKey {
    fn from(model: api_keys::Model) -> Self {
        Self {
            id: model.id as u64,
            user_id: model.user_id as u64,
            key_type: model.key_type,
            key_hash: model.key_hash,
            key_prefix: model.key_prefix,
            name: model.name,
            created_at: model.created_at.with_timezone(&chrono::Utc),
            last_used_at: model.last_used_at.map(|dt| dt.with_timezone(&chrono::Utc)),
        }
    }
}

#[async_trait::async_trait]
impl ApiKeyRepository for ApiKeyRepositoryAdapter {
    async fn create(&self, transaction: &dyn Transaction, new_key: NewApiKey, hash: String, prefix: String) -> Result<ApiKey, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;
        let mut model = api_keys::ActiveModel::new();
        model.user_id = Set(new_key.user_id as i64);
        model.key_type = Set(new_key.key_type);
        model.key_hash = Set(hash);
        model.key_prefix = Set(prefix);
        model.name = Set(new_key.name);
        model.last_used_at = Set(None);
        let model = model.insert(transaction).await.map_err(handle_dberr)?;
        Ok(model.into())
    }

    async fn find_by_id(&self, transaction: &dyn Transaction, id: ApiKeyId) -> Result<Option<ApiKey>, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::ApiKeys::find_by_id(id as i64)
            .one(transaction)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn find_by_hash(&self, transaction: &dyn Transaction, hash: &str) -> Result<Option<ApiKey>, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::ApiKeys::find()
            .filter(api_keys::Column::KeyHash.eq(hash))
            .one(transaction)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn list_for_user(&self, transaction: &dyn Transaction, user_id: UserId) -> Result<Vec<ApiKey>, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;
        Ok(ApiKeyEntity::find()
            .filter(api_keys::Column::UserId.eq(user_id as i64))
            .all(transaction)
            .await
            .map_err(handle_dberr)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn delete(&self, transaction: &dyn Transaction, id: ApiKeyId) -> Result<ApiKey, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;
        let existing = prelude::ApiKeys::find_by_id(id as i64)
            .one(transaction)
            .await
            .map_err(handle_dberr)?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let key: ApiKey = existing.clone().into();
        existing.delete(transaction).await.map_err(handle_dberr)?;
        Ok(key)
    }

    async fn delete_all_for_user(&self, transaction: &dyn Transaction, user_id: UserId) -> Result<Vec<ApiKey>, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;
        let existing = ApiKeyEntity::find()
            .filter(api_keys::Column::UserId.eq(user_id as i64))
            .all(transaction)
            .await
            .map_err(handle_dberr)?;
        let keys: Vec<ApiKey> = existing.iter().cloned().map(Into::into).collect();
        for model in existing {
            model.delete(transaction).await.map_err(handle_dberr)?;
        }
        Ok(keys)
    }

    async fn update_last_used(&self, transaction: &dyn Transaction, id: ApiKeyId) -> Result<(), Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;
        let existing = prelude::ApiKeys::find_by_id(id as i64)
            .one(transaction)
            .await
            .map_err(handle_dberr)?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let mut updater: api_keys::ActiveModel = existing.into();
        updater.last_used_at = Set(Some(Utc::now().into()));
        updater.update(transaction).await.map_err(handle_dberr)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{api_key::NewApiKey, repository::RepositoryService, user::NewUser};
    use sea_orm::Database;

    use crate::create_repository_service;

    async fn setup() -> Arc<RepositoryService> {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        create_repository_service(db).await.unwrap()
    }

    fn make_new_user(username: &str) -> NewUser {
        NewUser::new(
            username,
            "password123",
            format!("{username}@example.com"),
            format!("Test {username}"),
            ib_core::user::Capabilities::default(),
            false,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn create_and_find_by_hash() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();

        let user = svc.user_repository().create(&*tx, make_new_user("alice")).await.unwrap();
        let key = svc
            .api_key_repository()
            .create(
                &*tx,
                NewApiKey {
                    user_id: user.id,
                    key_type: "ib_live".to_owned(),
                    name: None,
                },
                "testhash".to_owned(),
                "ib_live_XXXX".to_owned(),
            )
            .await
            .unwrap();

        let found = svc.api_key_repository().find_by_hash(&*tx, "testhash").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, key.id);
    }

    #[tokio::test]
    async fn delete_all_for_user_removes_all_keys() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();

        let user = svc.user_repository().create(&*tx, make_new_user("bob")).await.unwrap();
        for i in 0..3u8 {
            svc.api_key_repository()
                .create(
                    &*tx,
                    NewApiKey {
                        user_id: user.id,
                        key_type: "ib_live".to_owned(),
                        name: None,
                    },
                    format!("hash{i}"),
                    "ib_live_XXXX".to_owned(),
                )
                .await
                .unwrap();
        }

        let deleted = svc.api_key_repository().delete_all_for_user(&*tx, user.id).await.unwrap();
        assert_eq!(deleted.len(), 3);

        let remaining = svc.api_key_repository().list_for_user(&*tx, user.id).await.unwrap();
        assert!(remaining.is_empty());
    }
}
