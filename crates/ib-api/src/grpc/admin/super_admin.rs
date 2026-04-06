pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        api_key::NewApiKey,
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
            false,
        )?;
        let user = svc.create_user(new_user).await?;

        let new_key = NewApiKey {
            user_id: user.id,
            key_type: "ib_live".to_owned(),
            name: Some("default".to_owned()),
        };
        let (key, plaintext) = core_services.api_key_service().create_key(new_key).await?;

        Ok(SuperAdminResponse {
            username: request.username,
            email: request.email,
            api_key: plaintext,
            key_prefix: key.key_prefix,
            key_type: key.key_type,
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
    use ib_core::{api_key::MockApiKeyRepository, user::MockUserRepository};
    use tonic::{Code, Request};

    use crate::grpc::{
        admin::tests::{fake_api_key, fake_user, make_service_with_repos},
        admin_proto::{SuperAdminRequest, admin_service_server::AdminService},
    };

    #[tokio::test]
    async fn super_admin_succeeds_when_no_existing_super_admin() {
        let user = fake_user(1, "alice");
        let key = fake_api_key(10, 1);
        let mut user_repo = MockUserRepository::new();
        let mut api_key_repo = MockApiKeyRepository::new();

        user_repo.expect_any_super_admin().returning(|_| Box::pin(async { Ok(false) }));
        user_repo.expect_create().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(u) })
        });
        api_key_repo.expect_create().returning(move |_, _, _, _| {
            let k = key.clone();
            Box::pin(async move { Ok(k) })
        });

        let svc = make_service_with_repos(user_repo, api_key_repo);
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
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_any_super_admin().returning(|_| Box::pin(async { Ok(true) }));
        let svc = make_service_with_repos(user_repo, MockApiKeyRepository::new());
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
