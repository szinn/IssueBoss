use ib_api::grpc::admin_proto::{ListProjectsRequest, admin_service_client::AdminServiceClient};
use ib_core::{
    api_key::NewApiKey,
    types::{Capabilities, Capability},
    user::NewUser,
};
use tonic::{metadata::MetadataValue, transport::Channel};

use crate::{grpc_context::GrpcTestContext, setup};

fn make_channel(addr: std::net::SocketAddr) -> Channel {
    let uri = format!("http://{addr}");
    Channel::from_shared(uri).unwrap().connect_lazy()
}

/// Create an Admin user and return its plaintext API key.
/// list_projects (and most gRPC endpoints) require Admin or SuperAdmin
/// capability.
async fn create_admin_key(services: &ib_core::CoreServices) -> String {
    let new_user = NewUser::new(
        "grpc-admin-user",
        "password123",
        "grpc-admin@example.com",
        "gRPC Admin",
        Capabilities(vec![Capability::Admin]),
        false,
    )
    .expect("valid new user");
    let user = services.user_service().create_user(new_user).await.unwrap();
    let new_key = NewApiKey {
        user_id: user.id,
        key_type: "ib_live".to_string(),
        name: None,
    };
    let (_key, plaintext) = services.api_key_service().create_key(new_key).await.unwrap();
    plaintext
}

/// A valid API key (for an Admin user) allows a ListProjects gRPC call through.
#[tokio::test]
async fn valid_api_key_accepted() {
    let ctx = setup().await;
    let plaintext = create_admin_key(&ctx.services).await;

    let grpc = GrpcTestContext::start(ctx.services.clone()).await;
    let mut client = AdminServiceClient::new(make_channel(grpc.addr));

    let mut req = tonic::Request::new(ListProjectsRequest {});
    req.metadata_mut().insert("x-api-key", MetadataValue::try_from(plaintext.as_str()).unwrap());

    let response = client.list_projects(req).await;
    assert!(response.is_ok(), "expected OK with valid key, got {response:?}");
}

/// A garbage API key must be rejected with UNAUTHENTICATED.
#[tokio::test]
async fn invalid_api_key_rejected() {
    let ctx = setup().await;
    let grpc = GrpcTestContext::start(ctx.services.clone()).await;
    let mut client = AdminServiceClient::new(make_channel(grpc.addr));

    let mut req = tonic::Request::new(ListProjectsRequest {});
    req.metadata_mut().insert("x-api-key", MetadataValue::try_from("garbage-key-value").unwrap());

    let err = client.list_projects(req).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated, "expected UNAUTHENTICATED, got {err:?}");
}

/// Missing API key must be rejected with UNAUTHENTICATED.
#[tokio::test]
async fn missing_api_key_rejected() {
    let ctx = setup().await;
    let grpc = GrpcTestContext::start(ctx.services.clone()).await;
    let mut client = AdminServiceClient::new(make_channel(grpc.addr));

    let req = tonic::Request::new(ListProjectsRequest {});
    // No x-api-key header

    let err = client.list_projects(req).await.unwrap_err();
    assert_eq!(err.code(), tonic::Code::Unauthenticated, "expected UNAUTHENTICATED, got {err:?}");
}
