pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        user::{Capabilities, NewUser},
    };

    use crate::grpc::{
        admin::user_to_proto,
        admin_proto::{
            ApiKeyResponse, CreateUserRequest, DeleteUserRequest, Empty, GetUserRequest, ListUsersRequest, ListUsersResponse, RotateApiKeyRequest,
            UpdateUserRequest, UserResponse,
        },
    };

    pub(crate) async fn create_user(core_services: &Arc<CoreServices>, req: CreateUserRequest) -> Result<UserResponse, Error> {
        let new_user = NewUser::new(&req.username, &req.password, &req.email, &req.full_name, Capabilities::default(), false)?;
        let user = core_services.user_service().create_user(new_user).await?;
        Ok(user_to_proto(user))
    }

    pub(crate) async fn get_user(core_services: &Arc<CoreServices>, req: GetUserRequest) -> Result<UserResponse, Error> {
        let user = core_services
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        Ok(user_to_proto(user))
    }

    pub(crate) async fn list_users(core_services: &Arc<CoreServices>, _req: ListUsersRequest) -> Result<ListUsersResponse, Error> {
        let users = core_services.user_service().list_users().await?;
        Ok(ListUsersResponse {
            users: users.into_iter().map(user_to_proto).collect(),
        })
    }

    pub(crate) async fn update_user(core_services: &Arc<CoreServices>, req: UpdateUserRequest) -> Result<UserResponse, Error> {
        let svc = core_services.user_service();
        let mut user = svc
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        if let Some(full_name) = req.full_name {
            user.full_name = full_name;
        }
        if let Some(email) = req.email {
            user.email_address = email;
        }
        let updated = svc.update_user(user).await?;
        Ok(user_to_proto(updated))
    }

    pub(crate) async fn delete_user(core_services: &Arc<CoreServices>, req: DeleteUserRequest) -> Result<Empty, Error> {
        let svc = core_services.user_service();
        let user = svc
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        svc.delete_user(user.id).await?;
        Ok(Empty {})
    }

    pub(crate) async fn rotate_api_key(core_services: &Arc<CoreServices>, req: RotateApiKeyRequest) -> Result<ApiKeyResponse, Error> {
        let (user, api_key) = core_services.user_service().rotate_api_key(&req.username).await?;
        Ok(ApiKeyResponse {
            username: user.username,
            api_key,
            api_key_prefix: user.api_key_prefix.unwrap_or_default(),
        })
    }
}

pub mod api {
    use ib_core::Error;

    use crate::{
        error::ApiError,
        grpc::{
            admin::api::{make_client, with_api_key},
            admin_proto::{
                ApiKeyResponse, CreateUserRequest, DeleteUserRequest, GetUserRequest, ListUsersRequest, RotateApiKeyRequest, UpdateUserRequest, UserResponse,
            },
        },
    };

    pub async fn create_user(host: &str, port: u16, username: &str, full_name: &str, email: &str, password: &str) -> Result<UserResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(CreateUserRequest {
            username: username.to_string(),
            full_name: full_name.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        }));
        Ok(client
            .create_user(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

    pub async fn get_user(host: &str, port: u16, username: &str) -> Result<UserResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(GetUserRequest {
            username: username.to_string(),
        }));
        Ok(client
            .get_user(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

    pub async fn list_users(host: &str, port: u16) -> Result<Vec<UserResponse>, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListUsersRequest {}));
        Ok(client
            .list_users(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner()
            .users)
    }

    pub async fn update_user(host: &str, port: u16, username: &str, full_name: Option<&str>, email: Option<&str>) -> Result<UserResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(UpdateUserRequest {
            username: username.to_string(),
            full_name: full_name.map(str::to_string),
            email: email.map(str::to_string),
        }));
        Ok(client
            .update_user(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

    pub async fn delete_user(host: &str, port: u16, username: &str) -> Result<(), Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(DeleteUserRequest {
            username: username.to_string(),
        }));
        client.delete_user(req).await.map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?;
        Ok(())
    }

    pub async fn rotate_api_key(host: &str, port: u16, username: &str) -> Result<ApiKeyResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(RotateApiKeyRequest {
            username: username.to_string(),
        }));
        Ok(client
            .rotate_api_key(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }
}

#[cfg(test)]
mod tests {
    use ib_core::user::MockUserRepository;
    use tonic::Code;

    use crate::grpc::{
        admin::{tests::make_core_services, user::handler},
        admin_proto::{CreateUserRequest, DeleteUserRequest, GetUserRequest, ListUsersRequest, RotateApiKeyRequest, UpdateUserRequest},
        error::map_core_error,
    };

    fn fake_user_for_test(id: u64, username: &str) -> ib_core::user::User {
        use chrono::Utc;
        ib_core::user::User {
            id,
            token: ib_core::user::UserToken::new(id),
            username: username.to_owned(),
            full_name: format!("Test {username}"),
            password_hash: "hash".to_owned(),
            email_address: format!("{username}@example.com"),
            api_key_hash: None,
            api_key_prefix: None,
            api_key_created_at: None,
            api_key_last_used_at: None,
            capabilities: ib_core::user::Capabilities::default(),
            change_password_on_login: false,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ── CRUD handlers — tested directly (auth is tested separately in auth.rs) ─

    #[tokio::test]
    async fn create_user_success() {
        let mut repo = MockUserRepository::new();
        let user = fake_user_for_test(1, "alice");
        repo.expect_create().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(u) })
        });
        let svc = make_core_services(repo);
        let resp = handler::create_user(
            &svc,
            CreateUserRequest {
                username: "alice".into(),
                full_name: "Alice".into(),
                email: "alice@example.com".into(),
                password: "pass".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.username, "alice");
    }

    #[tokio::test]
    async fn get_user_not_found_returns_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_core_services(repo);
        let err = handler::get_user(&svc, GetUserRequest { username: "ghost".into() })
            .await
            .map_err(map_core_error)
            .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_users_returns_empty_list() {
        let mut repo = MockUserRepository::new();
        repo.expect_list().returning(|_| Box::pin(async { Ok(vec![]) }));
        let svc = make_core_services(repo);
        let resp = handler::list_users(&svc, ListUsersRequest {}).await.unwrap();
        assert!(resp.users.is_empty());
    }

    #[tokio::test]
    async fn delete_user_not_found_returns_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_core_services(repo);
        let err = handler::delete_user(&svc, DeleteUserRequest { username: "ghost".into() })
            .await
            .map_err(map_core_error)
            .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn rotate_api_key_returns_plaintext_key() {
        let user = fake_user_for_test(1, "alice");
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        repo.expect_update().returning(|_, u| Box::pin(async move { Ok(u) }));
        let svc = make_core_services(repo);
        let resp = handler::rotate_api_key(&svc, RotateApiKeyRequest { username: "alice".into() }).await.unwrap();
        assert!(resp.api_key.starts_with("ib_live_"));
    }

    #[tokio::test]
    async fn update_user_success() {
        let user = fake_user_for_test(1, "alice");
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        repo.expect_update().returning(|_, u| Box::pin(async move { Ok(u) }));
        let svc = make_core_services(repo);
        let resp = handler::update_user(
            &svc,
            UpdateUserRequest {
                username: "alice".into(),
                full_name: Some("Alice Updated".into()),
                email: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.username, "alice");
    }

    #[tokio::test]
    async fn update_user_not_found_returns_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_core_services(repo);
        let err = handler::update_user(
            &svc,
            UpdateUserRequest {
                username: "ghost".into(),
                full_name: Some("Ghost".into()),
                email: None,
            },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn rotate_api_key_not_found_returns_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_core_services(repo);
        let err = handler::rotate_api_key(&svc, RotateApiKeyRequest { username: "ghost".into() })
            .await
            .map_err(map_core_error)
            .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }
}
