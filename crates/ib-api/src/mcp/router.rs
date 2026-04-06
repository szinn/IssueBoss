use std::sync::Arc;

use axum::{Router, middleware, routing::get};
use ib_core::CoreServices;

use super::handler::list_projects_handler;
use crate::auth::mcp_auth_middleware;

pub fn create_mcp_router(core_services: Arc<CoreServices>) -> Router {
    Router::new()
        .route("/mcp/list_projects", get(list_projects_handler))
        .layer(middleware::from_fn_with_state(core_services.clone(), mcp_auth_middleware))
        .with_state(core_services)
}
