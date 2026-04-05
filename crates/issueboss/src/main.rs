mod config;
mod health;

use std::net::SocketAddr;

use anyhow::Result;
use ib_api::{AdminServiceImpl, admin_proto::admin_service_server::AdminServiceServer, create_mcp_router};
use ib_frontend::create_frontend_router;
use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};
use tracing::info;

use crate::{config::Config, health::health_router};

async fn http_subsystem(subsys: SubsystemHandle) -> Result<()> {
    let cfg = Config::load()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.http_port));
    let router = create_frontend_router().merge(health_router());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("HTTP server listening on {addr}");
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            subsys.on_shutdown_requested().await;
        })
        .await?;
    Ok(())
}

async fn mcp_subsystem(subsys: SubsystemHandle) -> Result<()> {
    let cfg = Config::load()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.mcp_port));
    let router = create_mcp_router();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("MCP server listening on {addr}");
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            subsys.on_shutdown_requested().await;
        })
        .await?;
    Ok(())
}

async fn grpc_subsystem(subsys: SubsystemHandle) -> Result<()> {
    let cfg = Config::load()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.grpc_port));
    info!("gRPC server listening on {addr}");
    tonic::transport::Server::builder()
        .add_service(AdminServiceServer::new(AdminServiceImpl))
        .serve_with_shutdown(addr, async move {
            subsys.on_shutdown_requested().await;
        })
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    info!("issueboss starting…");

    Toplevel::new(|s| async move {
        s.start(SubsystemBuilder::new("http", http_subsystem));
        s.start(SubsystemBuilder::new("mcp", mcp_subsystem));
        s.start(SubsystemBuilder::new("grpc", grpc_subsystem));
    })
    .catch_signals()
    .handle_shutdown_requests(std::time::Duration::from_secs(5))
    .await
    .map_err(Into::into)
}
