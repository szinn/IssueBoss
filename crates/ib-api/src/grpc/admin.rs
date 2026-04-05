use tonic::{Request, Response, Status};

use crate::admin_proto::{SeedRequest, SeedResponse, admin_service_server::AdminService};

#[derive(Debug, Default)]
pub struct AdminServiceImpl;

#[tonic::async_trait]
impl AdminService for AdminServiceImpl {
    async fn seed(&self, _request: Request<SeedRequest>) -> Result<Response<SeedResponse>, Status> {
        Err(Status::already_exists("SuperAdmin already exists — use the admin API to manage users"))
    }
}

#[cfg(test)]
mod tests {
    use tonic::{Code, Request};

    use super::AdminServiceImpl;
    use crate::admin_proto::{SeedRequest, admin_service_server::AdminService};

    #[tokio::test]
    async fn seed_returns_already_exists() {
        let svc = AdminServiceImpl;
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
