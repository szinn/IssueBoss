use axum::{Router, routing::get};

async fn liveness() -> &'static str {
    r#"{"status":"ok"}"#
}

// TODO(IB-13): readiness check should verify DB connectivity
async fn readiness() -> &'static str {
    r#"{"status":"ok"}"#
}

pub fn health_router() -> Router {
    Router::new().route("/healthz", get(liveness)).route("/readyz", get(readiness))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::health_router;

    #[tokio::test]
    async fn health_live_returns_ok() {
        let app = health_router();
        let request = Request::builder().uri("/healthz").body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert_eq!(body_str, r#"{"status":"ok"}"#);
    }

    #[tokio::test]
    async fn health_ready_returns_ok() {
        let app = health_router();
        let request = Request::builder().uri("/readyz").body(Body::empty()).unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert_eq!(body_str, r#"{"status":"ok"}"#);
    }
}
