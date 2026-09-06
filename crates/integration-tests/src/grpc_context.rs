use std::sync::Arc;

use ib_api::grpc::{admin::GrpcAdminService, admin_proto::admin_service_server::AdminServiceServer};
use ib_core::CoreServices;
use tokio::sync::oneshot;
use tonic::transport::Server;

pub struct GrpcTestContext {
    pub addr: std::net::SocketAddr,
    _shutdown: oneshot::Sender<()>,
    _server_handle: tokio::task::JoinHandle<()>,
}

impl GrpcTestContext {
    /// Bind on a random OS port, start the gRPC server, and return the context.
    /// The server shuts down when this context is dropped.
    pub async fn start(core: Arc<CoreServices>) -> Self {
        // Bind to :0 to get an OS-assigned port, then release it so
        // serve_with_shutdown can reclaim the same port. The window is
        // tiny on loopback in tests.
        let addr = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap()
        };

        let admin_svc = GrpcAdminService::new(core);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let server_handle = tokio::spawn(async move {
            let _ = Server::builder()
                .add_service(AdminServiceServer::new(admin_svc))
                .serve_with_shutdown(addr, async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        // Give the server a moment to start listening.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        Self {
            addr,
            _shutdown: shutdown_tx,
            _server_handle: server_handle,
        }
    }
}
