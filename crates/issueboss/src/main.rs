mod config;

use anyhow::Result;
use ib_api::create_api_subsystem;
use ib_frontend::create_frontend_subsystem;
use tokio_graceful_shutdown::{SubsystemBuilder, Toplevel};
use tracing::info;

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")))
        .init();

    let cfg = Config::load()?;
    let (http_port, mcp_port, grpc_port) = (cfg.http_port, cfg.mcp_port, cfg.grpc_port);
    info!("issueboss starting…");

    Toplevel::new(move |s| async move {
        s.start(SubsystemBuilder::new("frontend", move |subsys| create_frontend_subsystem(http_port, subsys)));
        s.start(SubsystemBuilder::new("api", move |subsys| create_api_subsystem(mcp_port, grpc_port, subsys)));
    })
    .catch_signals()
    .handle_shutdown_requests(std::time::Duration::from_secs(5))
    .await
    .map_err(Into::into)
}
