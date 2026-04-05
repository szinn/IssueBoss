pub mod app;
mod health;

use std::net::SocketAddr;

use app::stay_tuned_html;
use axum::{Router, response::Html, routing::get};
use tokio_graceful_shutdown::SubsystemHandle;
use tracing::info;

pub fn create_frontend_router() -> Router {
    Router::new()
        .route("/", get(|| async { Html(stay_tuned_html()) }))
        .merge(health::health_router())
}

pub async fn create_frontend_subsystem(port: u16, subsys: SubsystemHandle) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("HTTP server listening on {addr}");
    axum::serve(listener, create_frontend_router())
        .with_graceful_shutdown(async move { subsys.on_shutdown_requested().await })
        .await?;
    Ok(())
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
