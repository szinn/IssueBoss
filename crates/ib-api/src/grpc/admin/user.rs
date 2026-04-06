pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        api_key::{ApiKey, NewApiKey},
        user::{Capabilities, NewUser},
    };

    use crate::grpc::{
        admin::user_to_proto,
        admin_proto::{
            ApiKeyEntry, CreateApiKeyRequest, CreateApiKeyResponse, CreateUserRequest, DeleteUserRequest, Empty, GetUserRequest, ListApiKeysRequest,
            ListApiKeysResponse, ListUsersRequest, ListUsersResponse, RevokeApiKeyRequest, UpdateUserRequest, UserResponse,
        },
    };

    fn api_key_to_entry(key: ApiKey) -> ApiKeyEntry {
        ApiKeyEntry {
            api_key_id: key.id,
            key_type: key.key_type,
            key_prefix: key.key_prefix,
            name: key.name.unwrap_or_default(),
            created_at: key.created_at.to_rfc3339(),
            last_used_at: key.last_used_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        }
    }

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
        core_services.api_key_service().revoke_all_for_user(user.id).await?;
        svc.delete_user(user.id).await?;
        Ok(Empty {})
    }

    pub(crate) async fn create_api_key(core_services: &Arc<CoreServices>, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse, Error> {
        let user = core_services
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let new_key = NewApiKey {
            user_id: user.id,
            key_type: req.key_type.clone(),
            name: if req.name.is_empty() { None } else { Some(req.name) },
        };
        let (key, plaintext) = core_services.api_key_service().create_key(new_key).await?;
        Ok(CreateApiKeyResponse {
            username: req.username,
            api_key: plaintext,
            key_prefix: key.key_prefix,
            key_type: key.key_type,
            api_key_id: key.id,
        })
    }

    pub(crate) async fn revoke_api_key(core_services: &Arc<CoreServices>, req: RevokeApiKeyRequest) -> Result<Empty, Error> {
        core_services.api_key_service().revoke_key(req.api_key_id).await?;
        Ok(Empty {})
    }

    pub(crate) async fn list_api_keys(core_services: &Arc<CoreServices>, req: ListApiKeysRequest) -> Result<ListApiKeysResponse, Error> {
        let user = core_services
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let keys = core_services.api_key_service().list_for_user(user.id).await?;
        Ok(ListApiKeysResponse {
            keys: keys.into_iter().map(api_key_to_entry).collect(),
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
                CreateApiKeyRequest, CreateApiKeyResponse, CreateUserRequest, DeleteUserRequest, GetUserRequest, ListApiKeysRequest, ListApiKeysResponse,
                ListUsersRequest, RevokeApiKeyRequest, UpdateUserRequest, UserResponse,
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

    pub async fn create_api_key(host: &str, port: u16, username: &str, key_type: &str, name: &str) -> Result<CreateApiKeyResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(CreateApiKeyRequest {
            username: username.to_string(),
            key_type: key_type.to_string(),
            name: name.to_string(),
        }));
        Ok(client
            .create_api_key(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

    pub async fn revoke_api_key(host: &str, port: u16, api_key_id: u64) -> Result<(), Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(RevokeApiKeyRequest { api_key_id }));
        client.revoke_api_key(req).await.map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?;
        Ok(())
    }

    pub async fn list_api_keys(host: &str, port: u16, username: &str) -> Result<ListApiKeysResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListApiKeysRequest {
            username: username.to_string(),
        }));
        Ok(client
            .list_api_keys(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }
}

#[cfg(test)]
mod tests {
    use ib_core::{
        api_key::{ApiKey, MockApiKeyRepository},
        user::MockUserRepository,
    };
    use tonic::Code;

    use crate::grpc::{
        admin::{tests::make_core_services, user::handler},
        admin_proto::{
            CreateApiKeyRequest, CreateUserRequest, DeleteUserRequest, GetUserRequest, ListApiKeysRequest, ListUsersRequest, RevokeApiKeyRequest,
            UpdateUserRequest,
        },
        error::map_core_error,
    };

    fn fake_user(id: u64, username: &str) -> ib_core::user::User {
        use chrono::Utc;
        ib_core::user::User {
            id,
            token: ib_core::user::UserToken::new(id),
            username: username.to_owned(),
            full_name: format!("Test {username}"),
            password_hash: "hash".to_owned(),
            email_address: format!("{username}@example.com"),
            capabilities: ib_core::user::Capabilities::default(),
            change_password_on_login: false,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn fake_api_key(id: u64, user_id: u64) -> ApiKey {
        use chrono::Utc;
        ApiKey {
            id,
            user_id,
            key_type: "ib_live".to_owned(),
            key_hash: "hash".to_owned(),
            key_prefix: "ib_live_XXXX".to_owned(),
            name: None,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    #[tokio::test]
    async fn create_user_success() {
        let user = fake_user(1, "alice");
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_create().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(u) })
        });
        let resp = handler::create_user(
            &make_core_services(user_repo, MockApiKeyRepository::new()),
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
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let err = handler::get_user(
            &make_core_services(user_repo, MockApiKeyRepository::new()),
            GetUserRequest { username: "ghost".into() },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_users_returns_empty_list() {
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_list().returning(|_| Box::pin(async { Ok(vec![]) }));
        let resp = handler::list_users(&make_core_services(user_repo, MockApiKeyRepository::new()), ListUsersRequest {})
            .await
            .unwrap();
        assert!(resp.users.is_empty());
    }

    #[tokio::test]
    async fn delete_user_not_found_returns_not_found() {
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let err = handler::delete_user(
            &make_core_services(user_repo, MockApiKeyRepository::new()),
            DeleteUserRequest { username: "ghost".into() },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn create_api_key_returns_plaintext() {
        let user = fake_user(1, "alice");
        let key = fake_api_key(10, 1);
        let mut user_repo = MockUserRepository::new();
        let mut api_key_repo = MockApiKeyRepository::new();
        user_repo.expect_find_by_username().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        api_key_repo.expect_create().returning(move |_, _, _, _| {
            let k = key.clone();
            Box::pin(async move { Ok(k) })
        });
        let resp = handler::create_api_key(
            &make_core_services(user_repo, api_key_repo),
            CreateApiKeyRequest {
                username: "alice".into(),
                key_type: "ib_live".into(),
                name: String::new(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.username, "alice");
        assert!(resp.api_key.starts_with("ib_live_"));
    }

    #[tokio::test]
    async fn revoke_api_key_not_found_returns_not_found() {
        let mut api_key_repo = MockApiKeyRepository::new();
        api_key_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let err = handler::revoke_api_key(
            &make_core_services(MockUserRepository::new(), api_key_repo),
            RevokeApiKeyRequest { api_key_id: 999 },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_api_keys_returns_keys_for_user() {
        let user = fake_user(1, "alice");
        let keys = vec![fake_api_key(10, 1), fake_api_key(11, 1)];
        let mut user_repo = MockUserRepository::new();
        let mut api_key_repo = MockApiKeyRepository::new();
        user_repo.expect_find_by_username().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        api_key_repo.expect_list_for_user().returning(move |_, _| {
            let k = keys.clone();
            Box::pin(async move { Ok(k) })
        });
        let resp = handler::list_api_keys(&make_core_services(user_repo, api_key_repo), ListApiKeysRequest { username: "alice".into() })
            .await
            .unwrap();
        assert_eq!(resp.keys.len(), 2);
    }

    #[tokio::test]
    async fn update_user_not_found_returns_not_found() {
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let err = handler::update_user(
            &make_core_services(user_repo, MockApiKeyRepository::new()),
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
}
