use std::sync::Arc;

use ib_core::CoreServices;
use tonic::{Request, Response, Status};

use crate::grpc::{
    admin_proto::{
        CreateApiKeyRequest, CreateApiKeyResponse, CreateUserRequest, DeleteUserRequest, Empty, GetUserRequest, ListApiKeysRequest, ListApiKeysResponse,
        ListUsersRequest, ListUsersResponse, RevokeApiKeyRequest, SuperAdminRequest, SuperAdminResponse, UpdateUserRequest, UserResponse,
        admin_service_server::AdminService,
    },
    error::map_core_error,
};

pub mod super_admin;
pub mod user;

fn user_to_proto(user: ib_core::user::User) -> UserResponse {
    UserResponse {
        token: user.token.to_string(),
        username: user.username,
        full_name: user.full_name,
        email: user.email_address,
        capabilities: user.capabilities.0.iter().map(|c| format!("{c:?}")).collect(),
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
        let response = super_admin::handler::super_admin(&self.core_services, request.into_inner())
            .await
            .map_err(map_core_error)?;
        Ok(Response::new(response))
    }

    async fn create_user(&self, request: Request<CreateUserRequest>) -> Result<Response<UserResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::create_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<UserResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::get_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn list_users(&self, request: Request<ListUsersRequest>) -> Result<Response<ListUsersResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::list_users(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn update_user(&self, request: Request<UpdateUserRequest>) -> Result<Response<UserResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::update_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn delete_user(&self, request: Request<DeleteUserRequest>) -> Result<Response<Empty>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::delete_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn create_api_key(&self, request: Request<CreateApiKeyRequest>) -> Result<Response<CreateApiKeyResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::create_api_key(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn revoke_api_key(&self, request: Request<RevokeApiKeyRequest>) -> Result<Response<Empty>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::revoke_api_key(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn list_api_keys(&self, request: Request<ListApiKeysRequest>) -> Result<Response<ListApiKeysResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::list_api_keys(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }
}

pub mod api {
    use ib_core::Error;

    use crate::{error::ApiError, grpc::admin_proto::admin_service_client::AdminServiceClient};

    pub(crate) async fn make_client(host: &str, port: u16) -> Result<AdminServiceClient<tonic::transport::Channel>, Error> {
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

    pub(crate) fn with_api_key<T>(mut req: tonic::Request<T>) -> tonic::Request<T> {
        if let Ok(key) = std::env::var("ISSUEBOSS_API_KEY") {
            if let Ok(val) = key.parse() {
                req.metadata_mut().insert("x-api-key", val);
            }
        }
        req
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{CoreServices, api_key::MockApiKeyRepository, create_services, user::MockUserRepository};

    use super::GrpcAdminService;

    pub(crate) fn make_core_services(user_repo: MockUserRepository, api_key_repo: MockApiKeyRepository) -> Arc<CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(user_repo))
                .api_key_repository(Arc::new(api_key_repo))
                .build()
                .unwrap(),
        );
        create_services(repo_svc)
    }

    pub(crate) fn make_service_with_repos(user_repo: MockUserRepository, api_key_repo: MockApiKeyRepository) -> GrpcAdminService {
        GrpcAdminService::new(make_core_services(user_repo, api_key_repo))
    }
}
