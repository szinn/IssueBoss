use std::{sync::Arc, time::Duration};

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

const MCP_SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(60 * 60);

fn make_session_manager() -> LocalSessionManager {
    let mut manager = LocalSessionManager::default();
    manager.session_config.keep_alive = Some(MCP_SESSION_IDLE_TIMEOUT);
    manager
}

pub fn create_mcp_router(core_services: Arc<CoreServices>) -> Router {
    let core = core_services.clone();
    let mcp_service = StreamableHttpService::new(
        move || {
            let user = CURRENT_MCP_USER
                .try_with(|u| u.clone())
                .map_err(|_| std::io::Error::other("no authenticated user in task context"))?;
            Ok(IssueBossServer::new(core.clone(), user))
        },
        Arc::new(make_session_manager()),
        StreamableHttpServerConfig::default().disable_allowed_hosts(),
    );
    Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(middleware::from_fn_with_state(core_services, mcp_auth_middleware))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_manager_uses_60_minute_idle_timeout() {
        let manager = make_session_manager();
        assert_eq!(manager.session_config.keep_alive, Some(MCP_SESSION_IDLE_TIMEOUT),);
        assert_eq!(MCP_SESSION_IDLE_TIMEOUT.as_secs(), 3600);
    }
}
