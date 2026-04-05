use axum::{Json, Router, routing::get};

use super::handler::{McpServer, ProjectSummary};

async fn list_projects() -> Json<Vec<ProjectSummary>> {
    Json(McpServer::dummy_projects())
}

pub fn create_mcp_router() -> Router {
    Router::new().route("/mcp/list_projects", get(list_projects))
}
