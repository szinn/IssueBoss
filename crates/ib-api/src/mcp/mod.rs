pub mod handler;
pub mod router;

use std::sync::Arc;

use ib_core::{CoreServices, Error};
pub use router::create_mcp_router;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};

use crate::error::ApiError;

pub(crate) struct McpSubsystem {
    port: u16,
    core_services: Arc<CoreServices>,
}

impl McpSubsystem {
    pub(crate) fn new(port: u16, core_services: Arc<CoreServices>) -> Self {
        Self { port, core_services }
    }
}

impl IntoSubsystem<Error> for McpSubsystem {
    async fn run(self, subsys: &mut SubsystemHandle) -> Result<(), Error> {
        let host_addr = format!("0.0.0.0:{}", self.port);
        let addr: std::net::SocketAddr = host_addr.parse().map_err(|_e| Error::from(ApiError::AddressParse(host_addr)))?;
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| Error::from(ApiError::Network(e.to_string())))?;
        tracing::info!("Listening on {addr}");
        tokio::select! {
            () = subsys.on_shutdown_requested() => {
                tracing::info!("McpSubsystem shutting down...");
            }
            result = axum::serve(listener, create_mcp_router()) => {
                result.map_err(|e| Error::from(ApiError::Network(e.to_string())))?;
                subsys.request_shutdown();
            }
        }
        Ok(())
    }
}
