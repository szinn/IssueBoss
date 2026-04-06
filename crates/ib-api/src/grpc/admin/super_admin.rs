pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        user::{Capabilities, Capability, NewUser},
    };

    use crate::grpc::admin_proto::{SuperAdminRequest, SuperAdminResponse};

    pub(crate) async fn super_admin(core_services: &Arc<CoreServices>, request: SuperAdminRequest) -> Result<SuperAdminResponse, Error> {
        let svc = core_services.user_service();

        if svc.any_super_admin().await? {
            return Err(Error::RepositoryError(RepositoryError::Conflict));
        }

        let new_user = NewUser::new(
            &request.username,
            &request.password,
            &request.email,
            &request.full_name,
            Capabilities(vec![Capability::SuperAdmin]),
            false, // admin supplies their own password — no forced change
        )?;
        svc.create_user(new_user).await?;
        let (_user, api_key) = svc.rotate_api_key(&request.username).await?;

        Ok(SuperAdminResponse {
            username: request.username,
            email: request.email,
            api_key,
        })
    }
}

pub mod api {
    use ib_core::Error;

    use crate::{
        error::ApiError,
        grpc::{
            admin::api::make_client,
            admin_proto::{SuperAdminRequest, SuperAdminResponse},
        },
    };

    /// No auth required — bootstrap endpoint. Creates the first super-admin and
    /// returns their API key.
    pub async fn super_admin(host: &str, port: u16, username: &str, full_name: &str, password: &str, email: &str) -> Result<SuperAdminResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = tonic::Request::new(SuperAdminRequest {
            username: username.to_string(),
            full_name: full_name.to_string(),
            password: password.to_string(),
            email: email.to_string(),
        });
        Ok(client
            .super_admin(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }
}

#[cfg(test)]
mod tests {
    // ── super_admin — exempt from auth, tested via AdminService ──────────────

    use ib_core::user::MockUserRepository;
    use tonic::{Code, Request};

    use crate::grpc::{
        admin::tests::make_service_with_repo,
        admin_proto::{SuperAdminRequest, admin_service_server::AdminService},
    };

    #[tokio::test]
    async fn super_admin_succeeds_when_no_existing_super_admin() {
        use ib_core::user::{Capabilities, Capability, User, UserToken};

        let mut repo = MockUserRepository::new();
        let now = chrono::Utc::now();

        // any_super_admin → false
        repo.expect_any_super_admin().returning(|_| Box::pin(async { Ok(false) }));

        // create → success
        repo.expect_create().returning(move |_, _| {
            Box::pin(async move {
                Ok(User {
                    id: 1,
                    token: UserToken::new(1),
                    username: "admin".to_owned(),
                    full_name: "Admin".to_owned(),
                    password_hash: "hash".to_owned(),
                    email_address: "admin@example.com".to_owned(),
                    api_key_hash: None,
                    api_key_prefix: None,
                    api_key_created_at: None,
                    api_key_last_used_at: None,
                    capabilities: Capabilities(vec![Capability::SuperAdmin]),
                    change_password_on_login: false,
                    version: 0,
                    created_at: now,
                    updated_at: now,
                })
            })
        });

        // find_by_username (called by rotate_api_key)
        repo.expect_find_by_username().returning(move |_, _| {
            Box::pin(async move {
                Ok(Some(User {
                    id: 1,
                    token: UserToken::new(1),
                    username: "admin".to_owned(),
                    full_name: "Admin".to_owned(),
                    password_hash: "hash".to_owned(),
                    email_address: "admin@example.com".to_owned(),
                    api_key_hash: None,
                    api_key_prefix: None,
                    api_key_created_at: None,
                    api_key_last_used_at: None,
                    capabilities: Capabilities(vec![Capability::SuperAdmin]),
                    change_password_on_login: false,
                    version: 0,
                    created_at: now,
                    updated_at: now,
                }))
            })
        });

        // update (called by rotate_api_key to save the key hash)
        repo.expect_update().returning(move |_, user| Box::pin(async move { Ok(user) }));

        let svc = make_service_with_repo(repo);
        let req = Request::new(SuperAdminRequest {
            username: "admin".into(),
            full_name: "Admin".into(),
            email: "admin@example.com".into(),
            password: "secret".into(),
        });
        let resp = svc.super_admin(req).await.unwrap().into_inner();
        assert_eq!(resp.username, "admin");
        assert!(resp.api_key.starts_with("ib_live_"));
    }

    #[tokio::test]
    async fn super_admin_fails_when_super_admin_already_exists() {
        let mut repo = MockUserRepository::new();
        repo.expect_any_super_admin().returning(|_| Box::pin(async { Ok(true) }));
        let svc = make_service_with_repo(repo);
        let req = Request::new(SuperAdminRequest {
            username: "admin".into(),
            full_name: "Admin".into(),
            email: "admin@example.com".into(),
            password: "secret".into(),
        });
        let err = svc.super_admin(req).await.unwrap_err();
        assert_eq!(err.code(), Code::AlreadyExists);
    }
}
