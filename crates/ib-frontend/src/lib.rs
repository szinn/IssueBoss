pub mod app;
mod health;

use std::{net::SocketAddr, sync::Arc};

use app::stay_tuned_html;
use axum::{Router, response::Html, routing::get};
use ib_core::CoreServices;
use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle};

pub struct FrontendSubsystem {
    port: u16,
}

pub fn create_frontend_router() -> Router {
    Router::new()
        .route("/", get(|| async { Html(stay_tuned_html()) }))
        .merge(health::health_router())
}

pub fn create_frontend_subsystem(port: u16, _core_services: Arc<CoreServices>) -> FrontendSubsystem {
    FrontendSubsystem { port }
}

impl IntoSubsystem<anyhow::Error> for FrontendSubsystem {
    async fn run(self, subsys: &mut SubsystemHandle) -> Result<(), anyhow::Error> {
        tracing::info!("FrontendSubsystem starting...");

        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("FrontendSubsystem started");
        tracing::info!("Listening on {addr}");

        tokio::select! {
            () = subsys.on_shutdown_requested() => {
                tracing::info!("Frontend shutting down...");
            }
            result = axum::serve(listener, create_frontend_router()) => {
                if let Err(e) = result {
                    tracing::error!("Frontend server error: {}", e);
                }
                subsys.request_shutdown();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::create_frontend_router;

    #[tokio::test]
    async fn root_returns_stay_tuned_html() {
        let app = create_frontend_router();
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(body_str.contains("Stay tuned"), "expected 'Stay tuned' in body");
    }
}
