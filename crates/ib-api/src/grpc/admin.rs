use tonic::{Request, Response, Status};

use crate::grpc::{
    admin_proto::{SeedRequest, SeedResponse, admin_service_server::AdminService},
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
    async fn seed(&self, request: Request<SeedRequest>) -> Result<Response<SeedResponse>, Status> {
        let response = handler::seed(request.into_inner()).await.map_err(map_core_error)?;
        Ok(Response::new(response))
    }
}

pub(crate) mod handler {
    use ib_core::{Error, RepositoryError};

    use crate::grpc::admin_proto::{SeedRequest, SeedResponse};

    pub(crate) async fn seed(_request: SeedRequest) -> Result<SeedResponse, Error> {
        Err(Error::RepositoryError(RepositoryError::Conflict))
    }
}

pub mod api {
    use ib_core::Error;

    use crate::{
        error::ApiError,
        grpc::admin_proto::{SeedRequest, SeedResponse, admin_service_client::AdminServiceClient},
    };

    pub async fn seed(host: &str, port: u16, username: &str, full_name: &str, password: &str, email: &str) -> Result<String, Error> {
        let uri = format!("{host}:{port}");
        let endpoint = tonic::transport::Channel::from_shared(uri.clone()).map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?;
        let endpoint = if uri.starts_with("https://") {
            let tls = tonic::transport::ClientTlsConfig::new().with_native_roots();
            endpoint.tls_config(tls).map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
        } else {
            endpoint
        };
        let mut client = AdminServiceClient::new(endpoint.connect().await.map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?);

        let request = SeedRequest {
            username: username.to_string(),
            full_name: full_name.to_string(),
            password: password.to_string(),
            email: email.to_string(),
        };
        let request = tonic::Request::new(request);
        let response: SeedResponse = client
            .seed(request)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner();

        Ok(response.api_key)
    }
}

#[cfg(test)]
mod tests {
    use tonic::{Code, Request};

    use super::GrpcAdminService;
    use crate::grpc::admin_proto::{SeedRequest, admin_service_server::AdminService};

    #[tokio::test]
    async fn seed_returns_already_exists() {
        let svc = GrpcAdminService::new();
        let req = Request::new(SeedRequest {
            username: "admin".into(),
            full_name: "Admin User".into(),
            email: "admin@example.com".into(),
            password: "s3cret".into(),
        });
        let err = svc.seed(req).await.unwrap_err();
        assert_eq!(err.code(), Code::AlreadyExists);
    }
}
