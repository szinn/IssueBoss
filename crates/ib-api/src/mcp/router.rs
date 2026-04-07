use std::sync::Arc;

use axum::{
    Router, middleware,
    routing::{get, post, put},
};
use ib_core::CoreServices;

use super::handler::{create_issue_handler, get_issue_handler, list_issues_handler, list_projects_handler, transition_issue_handler, update_issue_handler};
use crate::auth::mcp_auth_middleware;

pub fn create_mcp_router(core_services: Arc<CoreServices>) -> Router {
    Router::new()
        .route("/mcp/list_projects", get(list_projects_handler))
        .route("/mcp/issues", post(create_issue_handler))
        .route("/mcp/issues/{issue_slug}", get(get_issue_handler))
        .route("/mcp/issues/{issue_slug}", put(update_issue_handler))
        .route("/mcp/issues/{issue_slug}/transition", post(transition_issue_handler))
        .route("/mcp/projects/{project_token}/issues", get(list_issues_handler))
        .layer(middleware::from_fn_with_state(core_services.clone(), mcp_auth_middleware))
        .with_state(core_services)
}
