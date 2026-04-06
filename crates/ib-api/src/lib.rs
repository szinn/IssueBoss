mod error;
pub mod grpc;
mod mcp;

use std::sync::Arc;

use ib_core::{CoreServices, Error};
pub use mcp::create_mcp_router;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemBuilder, SubsystemHandle};

use crate::{grpc::GrpcSubsystem, mcp::McpSubsystem};

pub struct ApiSubsystem {
    grpc_port: u16,
    mcp_port: u16,
    core_services: Arc<CoreServices>,
}

impl IntoSubsystem<Error> for ApiSubsystem {
    async fn run(self, subsys: &mut SubsystemHandle) -> Result<(), Error> {
        tracing::info!("ApiSubsystem starting...");

        let grpc_subsystem = GrpcSubsystem::new(self.grpc_port, self.core_services.clone());
        let mcp_subsystem = McpSubsystem::new(self.mcp_port, self.core_services.clone());

        subsys.start(SubsystemBuilder::new("Grpc", grpc_subsystem.into_subsystem()));
        subsys.start(SubsystemBuilder::new("Mcp", mcp_subsystem.into_subsystem()));

        tracing::info!("ApiSubsystem started");

        subsys.on_shutdown_requested().await;
        tracing::info!("ApiSubsystem shutting down...");

        Ok(())
    }
}

#[must_use]
pub fn create_api_subsystem(grpc_port: u16, mcp_port: u16, core_services: Arc<CoreServices>) -> ApiSubsystem {
    ApiSubsystem {
        grpc_port,
        mcp_port,
        core_services,
    }
}
