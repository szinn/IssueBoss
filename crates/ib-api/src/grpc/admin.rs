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

    async fn create_user(&self, _request: Request<CreateUserRequest>) -> Result<Response<UserResponse>, Status> {
        Err(Status::unimplemented("TODO"))
    }

    async fn get_user(&self, _request: Request<GetUserRequest>) -> Result<Response<UserResponse>, Status> {
        Err(Status::unimplemented("TODO"))
    }

    async fn list_users(&self, _request: Request<ListUsersRequest>) -> Result<Response<ListUsersResponse>, Status> {
        Err(Status::unimplemented("TODO"))
    }

    async fn update_user(&self, _request: Request<UpdateUserRequest>) -> Result<Response<UserResponse>, Status> {
        Err(Status::unimplemented("TODO"))
    }

    async fn delete_user(&self, _request: Request<DeleteUserRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("TODO"))
    }

    async fn rotate_api_key(&self, _request: Request<RotateApiKeyRequest>) -> Result<Response<ApiKeyResponse>, Status> {
        Err(Status::unimplemented("TODO"))
    }
}

pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        user::{Capabilities, Capability, NewUser},
    };

    use crate::grpc::admin_proto::{SuperAdminRequest, SuperAdminResponse};

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
}

pub mod api {
    use ib_core::Error;

    use crate::{
        error::ApiError,
        grpc::admin_proto::{SuperAdminRequest, SuperAdminResponse, admin_service_client::AdminServiceClient},
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
    // Used in Task 6+ when user management RPCs are implemented.
    #[allow(dead_code)]
    fn with_api_key<T>(mut req: tonic::Request<T>) -> tonic::Request<T> {
        if let Ok(key) = std::env::var("ISSUEBOSS_API_KEY") {
            if let Ok(val) = key.parse() {
                req.metadata_mut().insert("x-api-key", val);
            }
        }
        req
    }

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
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{create_services, user::MockUserRepository};
    use tonic::{Code, Request};

    use super::GrpcAdminService;
    use crate::grpc::admin_proto::{SuperAdminRequest, admin_service_server::AdminService};

    fn make_service_with_repo(repo: MockUserRepository) -> GrpcAdminService {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(default_repository_service_builder().user_repository(Arc::new(repo)).build().unwrap());
        GrpcAdminService::new(create_services(repo_svc))
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
}
