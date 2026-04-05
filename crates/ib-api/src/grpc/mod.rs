use std::sync::Arc;

use ib_core::{CoreServices, Error};
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};
use tonic::transport::Server;

use crate::error::ApiError;

pub mod admin;
mod error;

pub(crate) mod admin_proto {
    tonic::include_proto!("issueboss.admin");
    pub(crate) const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("system_descriptor");
}

pub(crate) struct GrpcSubsystem {
    port: u16,
    core_services: Arc<CoreServices>,
}

impl GrpcSubsystem {
    pub(crate) fn new(port: u16, core_services: Arc<CoreServices>) -> Self {
        Self { port, core_services }
    }
}

impl IntoSubsystem<Error> for GrpcSubsystem {
    async fn run(self, subsys: &mut SubsystemHandle) -> Result<(), Error> {
        let host_addr = format!("0.0.0.0:{}", self.port);
        let addr = host_addr.parse().map_err(|_e| Error::from(ApiError::AddressParse(host_addr)))?;

        let service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(admin_proto::FILE_DESCRIPTOR_SET)
            .build_v1()
            .unwrap();
        let admin_service = admin::GrpcAdminService::new();

        tracing::info!("Listening on {}", addr);
        tokio::select! {
            () = subsys.on_shutdown_requested() => {
                tracing::info!("GrpcSubsystem shutting down...");
            }
            _ = Server::builder()
                .add_service(service)
                .add_service(admin_proto::admin_service_server::AdminServiceServer::new(admin_service))
                .serve(addr) => {
                subsys.request_shutdown();
            }
        }

        Ok(())
    }
}
