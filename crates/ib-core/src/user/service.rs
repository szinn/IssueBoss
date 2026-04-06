use std::sync::Arc;

use crate::{
    Error, RepositoryError,
    repository::RepositoryService,
    user::{NewUser, User, UserId, UserToken},
    with_read_only_transaction, with_transaction,
};

#[async_trait::async_trait]
pub trait UserService: Send + Sync {
    async fn create_user(&self, new_user: NewUser) -> Result<User, Error>;
    async fn update_user(&self, user: User) -> Result<User, Error>;
    async fn delete_user(&self, id: UserId) -> Result<User, Error>;
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, Error>;
    async fn find_by_token(&self, token: UserToken) -> Result<Option<User>, Error>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, Error>;
    async fn list_users(&self) -> Result<Vec<User>, Error>;
    async fn any_super_admin(&self) -> Result<bool, Error>;
}

pub(crate) struct UserServiceImpl {
    repository_service: Arc<RepositoryService>,
}

impl UserServiceImpl {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        Self { repository_service }
    }
}

#[async_trait::async_trait]
impl UserService for UserServiceImpl {
    async fn create_user(&self, new_user: NewUser) -> Result<User, Error> {
        with_transaction!(self, user_repository, |tx| user_repository.create(tx, new_user).await)
    }

    async fn update_user(&self, user: User) -> Result<User, Error> {
        with_transaction!(self, user_repository, |tx| user_repository.update(tx, user).await)
    }

    async fn delete_user(&self, id: UserId) -> Result<User, Error> {
        with_transaction!(self, user_repository, |tx| {
            let user = user_repository
                .find_by_id(tx, id)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            user_repository.delete(tx, user).await
        })
    }

    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, Error> {
        with_read_only_transaction!(self, user_repository, |tx| user_repository.find_by_id(tx, id).await)
    }

    async fn find_by_token(&self, token: UserToken) -> Result<Option<User>, Error> {
        with_read_only_transaction!(self, user_repository, |tx| user_repository.find_by_id(tx, token.id()).await)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, Error> {
        let username = username.to_owned();
        with_read_only_transaction!(self, user_repository, |tx| user_repository.find_by_username(tx, &username).await)
    }

    async fn list_users(&self) -> Result<Vec<User>, Error> {
        with_read_only_transaction!(self, user_repository, |tx| user_repository.list(tx).await)
    }

    async fn any_super_admin(&self) -> Result<bool, Error> {
        with_read_only_transaction!(self, user_repository, |tx| user_repository.any_super_admin(tx).await)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{UserService, UserServiceImpl};
    use crate::{
        Error, RepositoryError,
        repository::testing::default_repository_service_builder,
        user::{Capabilities, NewUser, User, UserToken, repository::MockUserRepository},
    };

    fn make_svc(user_repo: MockUserRepository) -> UserServiceImpl {
        let repo_svc = Arc::new(default_repository_service_builder().user_repository(Arc::new(user_repo)).build().unwrap());
        UserServiceImpl::new(repo_svc)
    }

    fn fake_user(id: u64, username: &str) -> User {
        use chrono::Utc;
        let now = Utc::now();
        User {
            id,
            token: UserToken::new(id),
            username: username.to_owned(),
            full_name: format!("Test {username}"),
            password_hash: "hash".to_owned(),
            email_address: format!("{username}@example.com"),
            capabilities: Capabilities::default(),
            change_password_on_login: false,
            version: 0,
            created_at: now,
            updated_at: now,
        }
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

    // ─── create_user ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_user_success() {
        let expected = fake_user(1, "alice");
        let mut repo = MockUserRepository::new();
        repo.expect_create().returning(move |_, _| {
            let u = expected.clone();
            Box::pin(async move { Ok(u) })
        });
        let result = make_svc(repo).create_user(make_new_user("alice")).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().username, "alice");
    }

    #[tokio::test]
    async fn create_user_propagates_constraint_error() {
        let mut repo = MockUserRepository::new();
        repo.expect_create()
            .returning(|_, _| Box::pin(async { Err(Error::RepositoryError(RepositoryError::Constraint("duplicate".into()))) }));
        let result = make_svc(repo).create_user(make_new_user("alice")).await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::Constraint(_)))));
    }

    // ─── find_by_id ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_by_id_found() {
        let user = fake_user(1, "alice");
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_id().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        let result = make_svc(repo).find_by_id(1).await;
        assert_eq!(result.unwrap().unwrap().username, "alice");
    }

    #[tokio::test]
    async fn find_by_id_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        assert!(make_svc(repo).find_by_id(999).await.unwrap().is_none());
    }

    // ─── find_by_token ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_by_token_delegates_to_find_by_id() {
        let user = fake_user(1, "alice");
        let token = user.token;
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_id().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        let result = make_svc(repo).find_by_token(token).await;
        assert_eq!(result.unwrap().unwrap().id, 1);
    }

    // ─── find_by_username ────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_by_username_found() {
        let user = fake_user(1, "alice");
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        let result = make_svc(repo).find_by_username("alice").await;
        assert_eq!(result.unwrap().unwrap().username, "alice");
    }

    // ─── delete_user ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_user_success() {
        let user = fake_user(1, "alice");
        let deleted = user.clone();
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_id().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        repo.expect_delete().returning(move |_, _| {
            let d = deleted.clone();
            Box::pin(async move { Ok(d) })
        });
        let result = make_svc(repo).delete_user(1).await;
        assert_eq!(result.unwrap().id, 1);
    }

    #[tokio::test]
    async fn delete_user_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let result = make_svc(repo).delete_user(999).await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    // ─── list_users ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_users_returns_all() {
        let users = vec![fake_user(1, "alice"), fake_user(2, "bob")];
        let expected = users.clone();
        let mut repo = MockUserRepository::new();
        repo.expect_list().returning(move |_| {
            let u = expected.clone();
            Box::pin(async move { Ok(u) })
        });
        let result = make_svc(repo).list_users().await.unwrap();
        assert_eq!(result.len(), 2);
    }

    // ─── any_super_admin ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn any_super_admin_delegates_to_repository() {
        let mut repo = MockUserRepository::new();
        repo.expect_any_super_admin().returning(|_| Box::pin(async { Ok(true) }));
        assert!(make_svc(repo).any_super_admin().await.unwrap());
    }
}
