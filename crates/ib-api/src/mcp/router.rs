use std::sync::Arc;

use axum::{Router, middleware};
use ib_core::CoreServices;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager,
    tower::{StreamableHttpServerConfig, StreamableHttpService},
};

use crate::{
    auth::{CURRENT_MCP_USER, mcp_auth_middleware},
    mcp::server::IssueBossServer,
};

pub fn create_mcp_router(core_services: Arc<CoreServices>) -> Router {
    let core = core_services.clone();
    let mcp_service = StreamableHttpService::new(
        move || {
            let user = CURRENT_MCP_USER
                .try_with(|u| u.clone())
                .map_err(|_| std::io::Error::other("no authenticated user in task context"))?;
            Ok(IssueBossServer::new(core.clone(), user))
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );
    Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(middleware::from_fn_with_state(core_services, mcp_auth_middleware))
}
