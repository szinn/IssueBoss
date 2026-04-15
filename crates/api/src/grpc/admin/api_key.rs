pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        api_key::{ApiKey, NewApiKey},
    };

    use crate::grpc::admin_proto::{
        ApiKeyEntry, CreateApiKeyRequest, CreateApiKeyResponse, Empty, ListApiKeysRequest, ListApiKeysResponse, RevokeApiKeyRequest,
    };

    fn api_key_to_entry(key: ApiKey) -> ApiKeyEntry {
        ApiKeyEntry {
            api_key_id: key.id,
            key_type: key.key_type,
            key_prefix: key.key_prefix,
            name: key.name.unwrap_or_default(),
            created_at: key.created_at.to_rfc3339(),
            last_used_at: key.last_used_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        }
    }

    pub(crate) async fn create_api_key(core_services: &Arc<CoreServices>, req: CreateApiKeyRequest) -> Result<CreateApiKeyResponse, Error> {
        let user = core_services
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let new_key = NewApiKey {
            user_id: user.id,
            key_type: req.key_type.clone(),
            name: if req.name.is_empty() { None } else { Some(req.name) },
        };
        let (key, plaintext) = core_services.api_key_service().create_key(new_key).await?;
        Ok(CreateApiKeyResponse {
            username: req.username,
            api_key: plaintext,
            key_prefix: key.key_prefix,
            key_type: key.key_type,
            api_key_id: key.id,
        })
    }

    pub(crate) async fn revoke_api_key(core_services: &Arc<CoreServices>, req: RevokeApiKeyRequest) -> Result<Empty, Error> {
        core_services.api_key_service().revoke_key(req.api_key_id).await?;
        Ok(Empty {})
    }

    pub(crate) async fn list_api_keys(core_services: &Arc<CoreServices>, req: ListApiKeysRequest) -> Result<ListApiKeysResponse, Error> {
        let user = core_services
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let keys = core_services.api_key_service().list_for_user(user.id).await?;
        Ok(ListApiKeysResponse {
            keys: keys.into_iter().map(api_key_to_entry).collect(),
        })
    }
}

pub mod api {
    use ib_core::Error;

    use crate::{
        error::ApiError,
        grpc::{
            admin::api::{make_client, with_api_key},
            admin_proto::{CreateApiKeyRequest, CreateApiKeyResponse, ListApiKeysRequest, ListApiKeysResponse, RevokeApiKeyRequest},
        },
    };

    pub async fn create_api_key(host: &str, port: u16, username: &str, key_type: &str, name: &str) -> Result<CreateApiKeyResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(CreateApiKeyRequest {
            username: username.to_string(),
            key_type: key_type.to_string(),
            name: name.to_string(),
        }));
        Ok(client
            .create_api_key(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

    pub async fn revoke_api_key(host: &str, port: u16, api_key_id: u64) -> Result<(), Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(RevokeApiKeyRequest { api_key_id }));
        client.revoke_api_key(req).await.map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?;
        Ok(())
    }

    pub async fn list_api_keys(host: &str, port: u16, username: &str) -> Result<ListApiKeysResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListApiKeysRequest {
            username: username.to_string(),
        }));
        Ok(client
            .list_api_keys(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }
}

#[cfg(test)]
mod tests {
    use ib_core::{api_key::MockApiKeyRepository, user::MockUserRepository};
    use tonic::Code;

    use crate::grpc::{
        admin::{
            api_key::handler,
            tests::{fake_api_key, fake_user, make_core_services},
        },
        admin_proto::{CreateApiKeyRequest, ListApiKeysRequest, RevokeApiKeyRequest},
        error::map_core_error,
    };

    #[tokio::test]
    async fn create_api_key_returns_plaintext() {
        let user = fake_user(1, "alice");
        let key = fake_api_key(10, 1);
        let mut user_repo = MockUserRepository::new();
        let mut api_key_repo = MockApiKeyRepository::new();
        user_repo.expect_find_by_username().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        api_key_repo.expect_create().returning(move |_, _, _, _| {
            let k = key.clone();
            Box::pin(async move { Ok(k) })
        });
        let resp = handler::create_api_key(
            &make_core_services(user_repo, api_key_repo),
            CreateApiKeyRequest {
                username: "alice".into(),
                key_type: "ib_live".into(),
                name: String::new(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.username, "alice");
        assert!(resp.api_key.starts_with("ib_live_"));
    }

    #[tokio::test]
    async fn revoke_api_key_not_found_returns_not_found() {
        let mut api_key_repo = MockApiKeyRepository::new();
        api_key_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let err = handler::revoke_api_key(
            &make_core_services(MockUserRepository::new(), api_key_repo),
            RevokeApiKeyRequest { api_key_id: 999 },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_api_keys_returns_keys_for_user() {
        let user = fake_user(1, "alice");
        let keys = vec![fake_api_key(10, 1), fake_api_key(11, 1)];
        let mut user_repo = MockUserRepository::new();
        let mut api_key_repo = MockApiKeyRepository::new();
        user_repo.expect_find_by_username().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(Some(u)) })
        });
        api_key_repo.expect_list_for_user().returning(move |_, _| {
            let k = keys.clone();
            Box::pin(async move { Ok(k) })
        });
        let resp = handler::list_api_keys(&make_core_services(user_repo, api_key_repo), ListApiKeysRequest { username: "alice".into() })
            .await
            .unwrap();
        assert_eq!(resp.keys.len(), 2);
    }
}
