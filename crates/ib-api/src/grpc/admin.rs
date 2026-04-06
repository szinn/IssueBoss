use tonic::{Request, Response, Status};

use crate::grpc::{
    admin_proto::{
        ApiKeyResponse, CreateUserRequest, DeleteUserRequest, Empty, GetUserRequest, ListUsersRequest, ListUsersResponse, RotateApiKeyRequest,
        SuperAdminRequest, SuperAdminResponse, UpdateUserRequest, UserResponse, admin_service_server::AdminService,
    },
    error::map_core_error,
};

pub struct GrpcAdminService;

impl GrpcAdminService {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[tonic::async_trait]
impl AdminService for GrpcAdminService {
    async fn super_admin(&self, request: Request<SuperAdminRequest>) -> Result<Response<SuperAdminResponse>, Status> {
        let response = handler::super_admin(request.into_inner()).await.map_err(map_core_error)?;
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
    use ib_core::{Error, RepositoryError};

    use crate::grpc::admin_proto::{SuperAdminRequest, SuperAdminResponse};

    pub(crate) async fn super_admin(_request: SuperAdminRequest) -> Result<SuperAdminResponse, Error> {
        Err(Error::RepositoryError(RepositoryError::Conflict))
    }
}

pub mod api {
    use ib_core::Error;

    use crate::{
        error::ApiError,
        grpc::admin_proto::{SuperAdminRequest, SuperAdminResponse, admin_service_client::AdminServiceClient},
    };

    async fn make_client(host: &str, port: u16) -> Result<AdminServiceClient<tonic::transport::Channel>, Error> {
        let uri = format!("{host}:{port}");
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
    use tonic::{Code, Request};

    use super::GrpcAdminService;
    use crate::grpc::admin_proto::{SuperAdminRequest, admin_service_server::AdminService};

    #[tokio::test]
    async fn super_admin_stub_returns_already_exists() {
        let svc = GrpcAdminService::new();
        let req = Request::new(SuperAdminRequest {
            username: "admin".into(),
            full_name: "Admin User".into(),
            email: "admin@example.com".into(),
            password: "s3cret".into(),
        });
        let err = svc.super_admin(req).await.unwrap_err();
        assert_eq!(err.code(), Code::AlreadyExists);
    }
}
