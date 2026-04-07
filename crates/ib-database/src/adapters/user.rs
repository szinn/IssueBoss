use chrono::Utc;
use ib_core::{
    Error, RepositoryError,
    repository::Transaction,
    user::{Capabilities, NewUser, User, UserId, UserToken, repository::UserRepository},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, QueryFilter, Set};

use crate::{
    entities::{
        prelude,
        users::{self, Entity as UserEntity},
    },
    handle_dberr,
    transaction::TransactionImpl,
};

pub(crate) struct UserRepositoryAdapter;

impl UserRepositoryAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl From<users::Model> for User {
    fn from(model: users::Model) -> Self {
        let token = UserToken::new(model.id as u64);
        let capabilities = serde_json::from_str(&model.capabilities).unwrap_or_default();

        Self {
            id: model.id as u64,
            token,
            username: model.username,
            full_name: model.full_name,
            password_hash: model.password_hash,
            email_address: model.email_address,
            capabilities,
            change_password_on_login: model.change_password_on_login,
            version: model.version as u64,
            created_at: model.created_at.with_timezone(&chrono::Utc),
            updated_at: model.updated_at.with_timezone(&chrono::Utc),
        }
    }
}

#[async_trait::async_trait]
impl UserRepository for UserRepositoryAdapter {
    async fn list(&self, transaction: &dyn Transaction) -> Result<Vec<User>, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;
        Ok(UserEntity::find()
            .all(transaction)
            .await
            .map_err(handle_dberr)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn find_by_id(&self, transaction: &dyn Transaction, id: UserId) -> Result<Option<User>, Error> {
        if id == 0 {
            return Err(Error::InvalidId(id));
        }
        let transaction = TransactionImpl::get_db_transaction(transaction)?;

        Ok(prelude::Users::find_by_id(id as i64)
            .one(transaction)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn find_by_username(&self, transaction: &dyn Transaction, username: &str) -> Result<Option<User>, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;

        Ok(prelude::Users::find()
            .filter(users::Column::Username.eq(username))
            .one(transaction)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn create(&self, transaction: &dyn Transaction, new_user: NewUser) -> Result<User, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;

        let token = UserToken::generate();
        let now = Utc::now();
        let capabilities = serde_json::to_string(&new_user.capabilities).map_err(|e| Error::Infrastructure(e.to_string()))?;

        let model = users::ActiveModel {
            id: Set(token.id() as i64),
            token: Set(token.to_string()),
            username: Set(new_user.username),
            full_name: Set(new_user.full_name),
            password_hash: Set(new_user.password_hash),
            email_address: Set(new_user.email_address),
            capabilities: Set(capabilities),
            change_password_on_login: Set(new_user.change_password_on_login),
            version: Set(0),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };
        let model = model.insert(transaction).await.map_err(handle_dberr)?;

        Ok(model.into())
    }

    async fn update(&self, transaction: &dyn Transaction, user: User) -> Result<User, Error> {
        if user.id == 0 {
            return Err(Error::InvalidId(user.id));
        }
        let transaction = TransactionImpl::get_db_transaction(transaction)?;

        let existing = prelude::Users::find_by_id(user.id as i64)
            .one(transaction)
            .await
            .map_err(handle_dberr)?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;

        if existing.version != user.version as i64 {
            return Err(Error::RepositoryError(RepositoryError::Conflict));
        }

        let mut updater: users::ActiveModel = existing.clone().into();

        if existing.username != user.username {
            updater.username = Set(user.username);
        }
        if existing.full_name != user.full_name {
            updater.full_name = Set(user.full_name);
        }
        if existing.email_address != user.email_address {
            updater.email_address = Set(user.email_address);
        }
        if existing.password_hash != user.password_hash {
            updater.password_hash = Set(user.password_hash);
        }
        let new_caps = serde_json::to_string(&user.capabilities).map_err(|e| Error::Infrastructure(e.to_string()))?;
        if existing.capabilities != new_caps {
            updater.capabilities = Set(new_caps);
        }
        if existing.change_password_on_login != user.change_password_on_login {
            updater.change_password_on_login = Set(user.change_password_on_login);
        }

        let result = updater.update(transaction).await.map_err(handle_dberr)?;
        Ok(result.into())
    }

    async fn delete(&self, transaction: &dyn Transaction, user: User) -> Result<User, Error> {
        if user.id == 0 {
            return Err(Error::InvalidId(user.id));
        }
        let transaction = TransactionImpl::get_db_transaction(transaction)?;

        let existing = prelude::Users::find_by_id(user.id as i64).one(transaction).await.map_err(handle_dberr)?;
        let Some(existing) = existing else {
            return Err(Error::RepositoryError(RepositoryError::NotFound));
        };

        if existing.version != user.version as i64 {
            return Err(Error::RepositoryError(RepositoryError::Conflict));
        }

        let user: User = existing.clone().into();
        existing.delete(transaction).await.map_err(handle_dberr)?;

        Ok(user)
    }

    async fn any_super_admin(&self, transaction: &dyn Transaction) -> Result<bool, Error> {
        let transaction = TransactionImpl::get_db_transaction(transaction)?;

        let users = UserEntity::find().all(transaction).await.map_err(handle_dberr)?;
        Ok(users.into_iter().any(|m| {
            let capabilities: Capabilities = serde_json::from_str(&m.capabilities).unwrap_or_default();
            capabilities.is_super_admin()
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{
        repository::RepositoryService,
        user::{Capabilities, Capability, NewUser},
    };
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
            Capabilities::default(),
            false,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn create_and_find_by_id() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();

        let new_user = make_new_user("alice");
        let created = svc.user_repository().create(&*tx, new_user).await.unwrap();
        let found = svc.user_repository().find_by_id(&*tx, created.id).await.unwrap().unwrap();
        assert_eq!(found.username, "alice");
    }

    #[tokio::test]
    async fn any_super_admin_false_when_empty() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();

        assert!(!svc.user_repository().any_super_admin(&*tx).await.unwrap());
    }

    #[tokio::test]
    async fn any_super_admin_true_after_create() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();

        let new_user = NewUser::new(
            "admin",
            "password123",
            "admin@example.com",
            "Admin User",
            Capabilities(vec![Capability::SuperAdmin]),
            false,
        )
        .unwrap();
        svc.user_repository().create(&*tx, new_user).await.unwrap();
        assert!(svc.user_repository().any_super_admin(&*tx).await.unwrap());
    }
}
