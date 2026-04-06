//! API key authentication shared between MCP (axum) and gRPC (Tonic) ports.

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use ib_core::{CoreServices, user::sha256_hex};
use tonic::Status;

/// A user successfully authenticated by API key.
/// Injected into axum request extensions by the MCP middleware,
/// or returned from `authenticate_grpc` for gRPC handlers.
#[derive(Clone)]
pub struct AuthenticatedUser(pub ib_core::user::User);

// ── MCP auth (axum middleware)
// ────────────────────────────────────────────────

/// Axum middleware that validates the `Authorization: Bearer <key>` header.
/// Injects `AuthenticatedUser` into request extensions on success.
pub async fn mcp_auth_middleware(State(core_services): State<Arc<CoreServices>>, mut req: Request, next: Next) -> Result<Response, StatusCode> {
    let key = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let hash = sha256_hex(key);
    let user = core_services
        .user_service()
        .find_by_api_key_hash(&hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(AuthenticatedUser(user));
    Ok(next.run(req).await)
}

// ── gRPC auth helper ─────────────────────────────────────────────────────────

/// Extract and validate the `x-api-key` gRPC metadata header.
/// Returns `Unauthenticated` if missing or invalid.
/// Does NOT check capabilities — callers check those themselves.
pub async fn authenticate_grpc(core_services: &Arc<CoreServices>, metadata: &tonic::metadata::MetadataMap) -> Result<ib_core::user::User, Status> {
    let key = metadata
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Status::unauthenticated("Missing x-api-key header"))?;

    let hash = sha256_hex(key);
    core_services
        .user_service()
        .find_by_api_key_hash(&hash)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| Status::unauthenticated("Invalid API key"))
}

/// Require that the authenticated user has `SuperAdmin` or `Admin` capability.
pub fn require_admin(user: &ib_core::user::User) -> Result<(), Status> {
    use ib_core::user::Capability;
    if user.capabilities.has(Capability::SuperAdmin) || user.capabilities.has(Capability::Admin) {
        Ok(())
    } else {
        Err(Status::permission_denied("SuperAdmin or Admin capability required"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn require_admin_rejects_plain_user() {
        use chrono::Utc;
        use ib_core::user::{Capabilities, User, UserToken};
        let user = User {
            id: 1,
            token: UserToken::new(1),
            username: "alice".to_owned(),
            full_name: "Alice".to_owned(),
            password_hash: "h".to_owned(),
            email_address: "a@b.com".to_owned(),
            api_key_hash: None,
            api_key_prefix: None,
            api_key_created_at: None,
            api_key_last_used_at: None,
            capabilities: Capabilities::default(),
            change_password_on_login: false,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let result = super::require_admin(&user);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::PermissionDenied);
    }

    #[test]
    fn require_admin_allows_super_admin() {
        use chrono::Utc;
        use ib_core::user::{Capabilities, Capability, User, UserToken};
        let user = User {
            id: 1,
            token: UserToken::new(1),
            username: "admin".to_owned(),
            full_name: "Admin".to_owned(),
            password_hash: "h".to_owned(),
            email_address: "a@b.com".to_owned(),
            api_key_hash: None,
            api_key_prefix: None,
            api_key_created_at: None,
            api_key_last_used_at: None,
            capabilities: Capabilities(vec![Capability::SuperAdmin]),
            change_password_on_login: false,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert!(super::require_admin(&user).is_ok());
    }
}
