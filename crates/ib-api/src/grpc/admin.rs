use std::sync::Arc;

use ib_core::CoreServices;
use tonic::{Request, Response, Status};

use crate::grpc::{
    admin_proto::{
        ApiKeyResponse, CreateUserRequest, DeleteUserRequest, Empty, GetUserRequest, ListUsersRequest, ListUsersResponse, RotateApiKeyRequest,
        SuperAdminRequest, SuperAdminResponse, UpdateUserRequest, UserResponse, admin_service_server::AdminService,
    },
    error::map_core_error,
};

fn user_to_proto(user: ib_core::user::User) -> UserResponse {
    UserResponse {
        token: user.token.to_string(),
        username: user.username,
        full_name: user.full_name,
        email: user.email_address,
        capabilities: user.capabilities.0.iter().map(|c| format!("{c:?}")).collect(),
        api_key_prefix: user.api_key_prefix.unwrap_or_default(),
        change_password_on_login: user.change_password_on_login,
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
    }
}

pub struct GrpcAdminService {
    core_services: Arc<CoreServices>,
}

impl GrpcAdminService {
    pub(crate) fn new(core_services: Arc<CoreServices>) -> Self {
        Self { core_services }
    }
}

#[tonic::async_trait]
impl AdminService for GrpcAdminService {
    async fn super_admin(&self, request: Request<SuperAdminRequest>) -> Result<Response<SuperAdminResponse>, Status> {
        let response = handler::super_admin(&self.core_services, request.into_inner()).await.map_err(map_core_error)?;
        Ok(Response::new(response))
    }

    async fn create_user(&self, request: Request<CreateUserRequest>) -> Result<Response<UserResponse>, Status> {
        handler::create_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<UserResponse>, Status> {
        handler::get_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn list_users(&self, request: Request<ListUsersRequest>) -> Result<Response<ListUsersResponse>, Status> {
        handler::list_users(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn update_user(&self, request: Request<UpdateUserRequest>) -> Result<Response<UserResponse>, Status> {
        handler::update_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn delete_user(&self, request: Request<DeleteUserRequest>) -> Result<Response<Empty>, Status> {
        handler::delete_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn rotate_api_key(&self, request: Request<RotateApiKeyRequest>) -> Result<Response<ApiKeyResponse>, Status> {
        handler::rotate_api_key(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }
}

pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        user::{Capabilities, Capability, NewUser},
    };

    use crate::grpc::admin_proto::{
        ApiKeyResponse, CreateUserRequest, DeleteUserRequest, Empty, GetUserRequest, ListUsersRequest, ListUsersResponse, RotateApiKeyRequest,
        SuperAdminRequest, SuperAdminResponse, UpdateUserRequest, UserResponse,
    };

    pub(crate) async fn super_admin(core_services: &Arc<CoreServices>, request: SuperAdminRequest) -> Result<SuperAdminResponse, Error> {
        let svc = core_services.user_service();

        if svc.any_super_admin().await? {
            return Err(Error::RepositoryError(RepositoryError::Conflict));
        }

        let new_user = NewUser::new(
            &request.username,
            &request.password,
            &request.email,
            &request.full_name,
            Capabilities(vec![Capability::SuperAdmin]),
            false, // admin supplies their own password — no forced change
        )?;
        svc.create_user(new_user).await?;
        let (_user, api_key) = svc.rotate_api_key(&request.username).await?;

        Ok(SuperAdminResponse {
            username: request.username,
            email: request.email,
            api_key,
        })
    }

    pub(crate) async fn create_user(core_services: &Arc<CoreServices>, req: CreateUserRequest) -> Result<UserResponse, Error> {
        let new_user = NewUser::new(&req.username, &req.password, &req.email, &req.full_name, Capabilities::default(), false)?;
        let user = core_services.user_service().create_user(new_user).await?;
        Ok(super::user_to_proto(user))
    }

    pub(crate) async fn get_user(core_services: &Arc<CoreServices>, req: GetUserRequest) -> Result<UserResponse, Error> {
        let user = core_services
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        Ok(super::user_to_proto(user))
    }

    pub(crate) async fn list_users(core_services: &Arc<CoreServices>, _req: ListUsersRequest) -> Result<ListUsersResponse, Error> {
        let users = core_services.user_service().list_users().await?;
        Ok(ListUsersResponse {
            users: users.into_iter().map(super::user_to_proto).collect(),
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
        Ok(super::user_to_proto(updated))
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
        grpc::admin_proto::{
            ApiKeyResponse, CreateUserRequest, DeleteUserRequest, GetUserRequest, ListUsersRequest, RotateApiKeyRequest, SuperAdminRequest, SuperAdminResponse,
            UpdateUserRequest, UserResponse, admin_service_client::AdminServiceClient,
        },
    };

    async fn make_client(host: &str, port: u16) -> Result<AdminServiceClient<tonic::transport::Channel>, Error> {
        let uri = format!("{}:{port}", host.trim_end_matches('/'));
        let endpoint = tonic::transport::Channel::from_shared(uri.clone()).map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?;
        let endpoint = if uri.starts_with("https://") {
            let tls = tonic::transport::ClientTlsConfig::new().with_native_roots();
            endpoint.tls_config(tls).map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
        } else {
            endpoint
        };
        Ok(AdminServiceClient::new(
            endpoint.connect().await.map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?,
        ))
    }

    // Helper to attach API key header (no-op when env var is unset).
    fn with_api_key<T>(mut req: tonic::Request<T>) -> tonic::Request<T> {
        if let Ok(key) = std::env::var("ISSUEBOSS_API_KEY") {
            if let Ok(val) = key.parse() {
                req.metadata_mut().insert("x-api-key", val);
            }
        }
        req
    }

    /// No auth required — bootstrap endpoint. Creates the first super-admin and
    /// returns their API key.
    pub async fn super_admin(host: &str, port: u16, username: &str, full_name: &str, password: &str, email: &str) -> Result<SuperAdminResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = tonic::Request::new(SuperAdminRequest {
            username: username.to_string(),
            full_name: full_name.to_string(),
            password: password.to_string(),
            email: email.to_string(),
        });
        Ok(client
            .super_admin(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

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
    use std::sync::Arc;

    use ib_core::{create_services, user::MockUserRepository};
    use tonic::{Code, Request};

    use super::GrpcAdminService;
    use crate::grpc::admin_proto::{
        CreateUserRequest, DeleteUserRequest, GetUserRequest, ListUsersRequest, RotateApiKeyRequest, SuperAdminRequest, UpdateUserRequest,
        admin_service_server::AdminService,
    };

    fn make_service_with_repo(repo: MockUserRepository) -> GrpcAdminService {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(default_repository_service_builder().user_repository(Arc::new(repo)).build().unwrap());
        GrpcAdminService::new(create_services(repo_svc))
    }

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

    #[tokio::test]
    async fn super_admin_succeeds_when_no_existing_super_admin() {
        use ib_core::user::{Capabilities, Capability, User, UserToken};

        let mut repo = MockUserRepository::new();
        let now = chrono::Utc::now();

        // any_super_admin → false
        repo.expect_any_super_admin().returning(|_| Box::pin(async { Ok(false) }));

        // create → success
        repo.expect_create().returning(move |_, _| {
            Box::pin(async move {
                Ok(User {
                    id: 1,
                    token: UserToken::new(1),
                    username: "admin".to_owned(),
                    full_name: "Admin".to_owned(),
                    password_hash: "hash".to_owned(),
                    email_address: "admin@example.com".to_owned(),
                    api_key_hash: None,
                    api_key_prefix: None,
                    api_key_created_at: None,
                    api_key_last_used_at: None,
                    capabilities: Capabilities(vec![Capability::SuperAdmin]),
                    change_password_on_login: false,
                    version: 0,
                    created_at: now,
                    updated_at: now,
                })
            })
        });

        // find_by_username (called by rotate_api_key)
        repo.expect_find_by_username().returning(move |_, _| {
            Box::pin(async move {
                Ok(Some(User {
                    id: 1,
                    token: UserToken::new(1),
                    username: "admin".to_owned(),
                    full_name: "Admin".to_owned(),
                    password_hash: "hash".to_owned(),
                    email_address: "admin@example.com".to_owned(),
                    api_key_hash: None,
                    api_key_prefix: None,
                    api_key_created_at: None,
                    api_key_last_used_at: None,
                    capabilities: Capabilities(vec![Capability::SuperAdmin]),
                    change_password_on_login: false,
                    version: 0,
                    created_at: now,
                    updated_at: now,
                }))
            })
        });

        // update (called by rotate_api_key to save the key hash)
        repo.expect_update().returning(move |_, user| Box::pin(async move { Ok(user) }));

        let svc = make_service_with_repo(repo);
        let req = Request::new(SuperAdminRequest {
            username: "admin".into(),
            full_name: "Admin".into(),
            email: "admin@example.com".into(),
            password: "secret".into(),
        });
        let resp = svc.super_admin(req).await.unwrap().into_inner();
        assert_eq!(resp.username, "admin");
        assert!(resp.api_key.starts_with("ib_live_"));
    }

    #[tokio::test]
    async fn super_admin_fails_when_super_admin_already_exists() {
        let mut repo = MockUserRepository::new();
        repo.expect_any_super_admin().returning(|_| Box::pin(async { Ok(true) }));
        let svc = make_service_with_repo(repo);
        let req = Request::new(SuperAdminRequest {
            username: "admin".into(),
            full_name: "Admin".into(),
            email: "admin@example.com".into(),
            password: "secret".into(),
        });
        let err = svc.super_admin(req).await.unwrap_err();
        assert_eq!(err.code(), Code::AlreadyExists);
    }

    #[tokio::test]
    async fn create_user_success() {
        let mut repo = MockUserRepository::new();
        let user = fake_user_for_test(1, "alice");
        repo.expect_create().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(u) })
        });
        let svc = make_service_with_repo(repo);
        let req = Request::new(CreateUserRequest {
            username: "alice".into(),
            full_name: "Alice".into(),
            email: "alice@example.com".into(),
            password: "pass".into(),
        });
        let resp = svc.create_user(req).await.unwrap().into_inner();
        assert_eq!(resp.username, "alice");
    }

    #[tokio::test]
    async fn get_user_not_found_returns_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_service_with_repo(repo);
        let req = Request::new(GetUserRequest { username: "ghost".into() });
        let err = svc.get_user(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_users_returns_empty_list() {
        let mut repo = MockUserRepository::new();
        repo.expect_list().returning(|_| Box::pin(async { Ok(vec![]) }));
        let svc = make_service_with_repo(repo);
        let req = Request::new(ListUsersRequest {});
        let resp = svc.list_users(req).await.unwrap().into_inner();
        assert!(resp.users.is_empty());
    }

    #[tokio::test]
    async fn delete_user_not_found_returns_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_service_with_repo(repo);
        let req = Request::new(DeleteUserRequest { username: "ghost".into() });
        let err = svc.delete_user(req).await.unwrap_err();
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
        let svc = make_service_with_repo(repo);
        let req = Request::new(RotateApiKeyRequest { username: "alice".into() });
        let resp = svc.rotate_api_key(req).await.unwrap().into_inner();
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
        let svc = make_service_with_repo(repo);
        let req = Request::new(UpdateUserRequest {
            username: "alice".into(),
            full_name: Some("Alice Updated".into()),
            email: None,
        });
        let resp = svc.update_user(req).await.unwrap().into_inner();
        assert_eq!(resp.username, "alice");
    }

    #[tokio::test]
    async fn update_user_not_found_returns_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_service_with_repo(repo);
        let req = Request::new(UpdateUserRequest {
            username: "ghost".into(),
            full_name: Some("Ghost".into()),
            email: None,
        });
        let err = svc.update_user(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn rotate_api_key_not_found_returns_not_found() {
        let mut repo = MockUserRepository::new();
        repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_service_with_repo(repo);
        let req = Request::new(RotateApiKeyRequest { username: "ghost".into() });
        let err = svc.rotate_api_key(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }
}
