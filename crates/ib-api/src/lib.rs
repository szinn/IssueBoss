pub mod admin_proto {
    tonic::include_proto!("issueboss.admin.v1");
}

pub mod grpc;
pub mod mcp;

use std::net::SocketAddr;

use admin_proto::admin_service_server::AdminServiceServer;
pub use grpc::AdminServiceImpl;
pub use mcp::create_mcp_router;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle};
use tracing::info;

async fn grpc_subsystem(port: u16, subsys: SubsystemHandle) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("gRPC server listening on {addr}");
    tonic::transport::Server::builder()
        .add_service(AdminServiceServer::new(AdminServiceImpl))
        .serve_with_shutdown(addr, async move { subsys.on_shutdown_requested().await })
        .await?;
    Ok(())
}

async fn mcp_subsystem(port: u16, subsys: SubsystemHandle) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("MCP server listening on {addr}");
    axum::serve(listener, create_mcp_router())
        .with_graceful_shutdown(async move { subsys.on_shutdown_requested().await })
        .await?;
    Ok(())
}

pub async fn create_api_subsystem(mcp_port: u16, grpc_port: u16, subsys: SubsystemHandle) -> anyhow::Result<()> {
    subsys.start(SubsystemBuilder::new("mcp", move |s| mcp_subsystem(mcp_port, s)));
    subsys.start(SubsystemBuilder::new("grpc", move |s| grpc_subsystem(grpc_port, s)));
    subsys.on_shutdown_requested().await;
    Ok(())
}
