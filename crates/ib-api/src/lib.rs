pub mod admin_proto {
    tonic::include_proto!("issueboss.admin.v1");
}

pub mod grpc;
pub mod mcp;

use std::net::SocketAddr;

use admin_proto::admin_service_server::AdminServiceServer;
pub use grpc::AdminServiceImpl;
pub use mcp::create_mcp_router;

pub async fn serve_grpc(addr: SocketAddr) -> Result<(), tonic::transport::Error> {
    tonic::transport::Server::builder()
        .add_service(AdminServiceServer::new(AdminServiceImpl))
        .serve(addr)
        .await
}
