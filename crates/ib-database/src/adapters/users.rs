use ib_core::{
    RepositoryError,
    user::{Capabilities, User, UserId, UserToken, repository::UserRepository},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entities::users::{self, ActiveModel, Entity as UserEntity};

pub struct UserRepositoryImpl {
    db: DatabaseConnection,
}

impl UserRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn model_to_domain(m: users::Model) -> User {
    User {
        id: m.id as u64,
        token: UserToken::from_id(m.id as u64),
        username: m.username,
        full_name: m.full_name,
        password_hash: m.password_hash,
        email_address: m.email_address,
        api_key_hash: m.api_key_hash,
        api_key_prefix: m.api_key_prefix,
        api_key_created_at: m.api_key_created_at.map(|dt| dt.with_timezone(&chrono::Utc)),
        api_key_last_used_at: m.api_key_last_used_at.map(|dt| dt.with_timezone(&chrono::Utc)),
        capabilities: serde_json::from_value(m.capabilities).unwrap_or_else(|e| {
            tracing::error!(
                error = %e,
                "failed to deserialize capabilities for user id {}; defaulting to empty",
                m.id,
            );
            Capabilities::default()
        }),
        created_at: m.created_at.with_timezone(&chrono::Utc),
        updated_at: m.updated_at.with_timezone(&chrono::Utc),
    }
}

fn domain_to_active(user: &User) -> ActiveModel {
    ActiveModel {
        id: Set(user.id as i64),
        token: Set(user.token.to_string()),
        username: Set(user.username.clone()),
        full_name: Set(user.full_name.clone()),
        password_hash: Set(user.password_hash.clone()),
        email_address: Set(user.email_address.clone()),
        api_key_hash: Set(user.api_key_hash.clone()),
        api_key_prefix: Set(user.api_key_prefix.clone()),
        api_key_created_at: Set(user.api_key_created_at.map(|dt| dt.fixed_offset())),
        api_key_last_used_at: Set(user.api_key_last_used_at.map(|dt| dt.fixed_offset())),
        capabilities: Set(serde_json::to_value(&user.capabilities).expect("Capabilities serializes to a JSON array — this cannot fail")),
        created_at: Set(user.created_at.fixed_offset()),
        updated_at: Set(user.updated_at.fixed_offset()),
    }
}

fn db_err(e: sea_orm::DbErr) -> RepositoryError {
    RepositoryError::Database(e.to_string())
}

impl UserRepository for UserRepositoryImpl {
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        UserEntity::find_by_id(id as i64)
            .one(&self.db)
            .await
            .map_err(db_err)
            .map(|opt| opt.map(model_to_domain))
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, RepositoryError> {
        UserEntity::find()
            .filter(users::Column::Username.eq(username))
            .one(&self.db)
            .await
            .map_err(db_err)
            .map(|opt| opt.map(model_to_domain))
    }

    async fn find_by_api_key_hash(&self, hash: &str) -> Result<Option<User>, RepositoryError> {
        UserEntity::find()
            .filter(users::Column::ApiKeyHash.eq(hash))
            .one(&self.db)
            .await
            .map_err(db_err)
            .map(|opt| opt.map(model_to_domain))
    }

    async fn create(&self, user: User) -> Result<User, RepositoryError> {
        let active = domain_to_active(&user);
        active.insert(&self.db).await.map_err(db_err)?;
        self.find_by_id(user.id).await?.ok_or(RepositoryError::NotFound)
    }

    async fn update(&self, user: User) -> Result<User, RepositoryError> {
        let active = domain_to_active(&user);
        active.update(&self.db).await.map_err(db_err)?;
        self.find_by_id(user.id).await?.ok_or(RepositoryError::NotFound)
    }

    async fn delete(&self, id: UserId) -> Result<(), RepositoryError> {
        UserEntity::delete_by_id(id as i64).exec(&self.db).await.map_err(db_err).map(|_| ())
    }

    async fn any_super_admin(&self) -> Result<bool, RepositoryError> {
        // Load all users and check capabilities in Rust to avoid DB-specific JSON query
        // syntax.
        let users = UserEntity::find().all(&self.db).await.map_err(db_err)?;
        Ok(users.into_iter().any(|m| {
            let caps: Capabilities = serde_json::from_value(m.capabilities).unwrap_or_else(|e| {
                tracing::error!(
                    error = %e,
                    "failed to deserialize capabilities for user id {} in any_super_admin; defaulting to empty",
                    m.id,
                );
                Capabilities::default()
            });
            caps.is_super_admin()
        }))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use ib_core::user::{Capabilities, Capability, User, UserToken, repository::UserRepository};
    use sea_orm::{Database, DatabaseConnection};
    use sea_orm_migration::MigratorTrait;

    use super::UserRepositoryImpl;
    use crate::migrator::Migrator;

    async fn setup() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        Migrator::up(&db, None).await.unwrap();
        db
    }

    fn make_user(id: u64, username: &str) -> User {
        let now = Utc::now();
        User {
            id,
            token: UserToken::from_id(id),
            username: username.to_owned(),
            full_name: format!("Test {username}"),
            password_hash: "hash".to_owned(),
            email_address: format!("{username}@example.com"),
            api_key_hash: None,
            api_key_prefix: None,
            api_key_created_at: None,
            api_key_last_used_at: None,
            capabilities: Capabilities::default(),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_and_find_by_id() {
        let db = setup().await;
        let repo = UserRepositoryImpl::new(db);
        let user = make_user(1, "alice");
        let created = repo.create(user).await.unwrap();
        let found = repo.find_by_id(created.id).await.unwrap().unwrap();
        assert_eq!(found.username, "alice");
    }

    #[tokio::test]
    async fn any_super_admin_false_when_empty() {
        let db = setup().await;
        let repo = UserRepositoryImpl::new(db);
        assert!(!repo.any_super_admin().await.unwrap());
    }

    #[tokio::test]
    async fn any_super_admin_true_after_create() {
        let db = setup().await;
        let repo = UserRepositoryImpl::new(db);
        let mut user = make_user(1, "admin");
        user.capabilities = Capabilities(vec![Capability::SuperAdmin]);
        repo.create(user).await.unwrap();
        assert!(repo.any_super_admin().await.unwrap());
    }
}
