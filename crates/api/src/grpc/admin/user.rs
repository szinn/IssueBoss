pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{CoreServices, Error, RepositoryError, types::Capabilities, user::NewUser};

    use crate::grpc::{
        admin::user_to_proto,
        admin_proto::{CreateUserRequest, DeleteUserRequest, Empty, GetUserRequest, ListUsersRequest, ListUsersResponse, UpdateUserRequest, UserResponse},
    };

    pub(crate) async fn create_user(core_services: &Arc<CoreServices>, req: CreateUserRequest) -> Result<UserResponse, Error> {
        let new_user = NewUser::new(&req.username, &req.password, &req.email, &req.full_name, Capabilities::default(), false)?;
        let user = core_services.user_service().create_user(new_user).await?;
        Ok(user_to_proto(user))
    }

    pub(crate) async fn get_user(core_services: &Arc<CoreServices>, req: GetUserRequest) -> Result<UserResponse, Error> {
        let user = core_services
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        Ok(user_to_proto(user))
    }

    pub(crate) async fn list_users(core_services: &Arc<CoreServices>, _req: ListUsersRequest) -> Result<ListUsersResponse, Error> {
        let users = core_services.user_service().list_users().await?;
        Ok(ListUsersResponse {
            users: users.into_iter().map(user_to_proto).collect(),
        })
    }

    pub(crate) async fn update_user(core_services: &Arc<CoreServices>, req: UpdateUserRequest) -> Result<UserResponse, Error> {
        let svc = core_services.user_service();
        let mut user = svc
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        if let Some(full_name) = req.full_name {
            user.full_name = full_name;
        }
        if let Some(email) = req.email {
            user.email_address = email;
        }
        let updated = svc.update_user(user).await?;
        Ok(user_to_proto(updated))
    }

    pub(crate) async fn delete_user(core_services: &Arc<CoreServices>, req: DeleteUserRequest) -> Result<Empty, Error> {
        let svc = core_services.user_service();
        let user = svc
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        core_services.api_key_service().revoke_all_for_user(user.id).await?;
        svc.delete_user(user.id).await?;
        Ok(Empty {})
    }
}

pub mod api {
    use ib_core::Error;

    use crate::{
        error::ApiError,
        grpc::{
            admin::api::{make_client, with_api_key},
            admin_proto::{CreateUserRequest, DeleteUserRequest, GetUserRequest, ListUsersRequest, UpdateUserRequest, UserResponse},
        },
    };

    pub async fn create_user(host: &str, port: u16, username: &str, full_name: &str, email: &str, password: &str) -> Result<UserResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(CreateUserRequest {
            username: username.to_string(),
            full_name: full_name.to_string(),
            email: email.to_string(),
            password: password.to_string(),
        }));
        Ok(client
            .create_user(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

    pub async fn get_user(host: &str, port: u16, username: &str) -> Result<UserResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(GetUserRequest {
            username: username.to_string(),
        }));
        Ok(client
            .get_user(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

    pub async fn list_users(host: &str, port: u16) -> Result<Vec<UserResponse>, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListUsersRequest {}));
        Ok(client
            .list_users(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner()
            .users)
    }

    pub async fn update_user(host: &str, port: u16, username: &str, full_name: Option<&str>, email: Option<&str>) -> Result<UserResponse, Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(UpdateUserRequest {
            username: username.to_string(),
            full_name: full_name.map(str::to_string),
            email: email.map(str::to_string),
        }));
        Ok(client
            .update_user(req)
            .await
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
            .into_inner())
    }

    pub async fn delete_user(host: &str, port: u16, username: &str) -> Result<(), Error> {
        let mut client = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(DeleteUserRequest {
            username: username.to_string(),
        }));
        client.delete_user(req).await.map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ib_core::{api_key::MockApiKeyRepository, user::MockUserRepository};
    use tonic::Code;

    use crate::grpc::{
        admin::{
            tests::{fake_user, make_core_services},
            user::handler,
        },
        admin_proto::{CreateUserRequest, DeleteUserRequest, GetUserRequest, ListUsersRequest, UpdateUserRequest},
        error::map_core_error,
    };

    #[tokio::test]
    async fn create_user_success() {
        let user = fake_user(1, "alice");
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_create().returning(move |_, _| {
            let u = user.clone();
            Box::pin(async move { Ok(u) })
        });
        let resp = handler::create_user(
            &make_core_services(user_repo, MockApiKeyRepository::new()),
            CreateUserRequest {
                username: "alice".into(),
                full_name: "Alice".into(),
                email: "alice@example.com".into(),
                password: "pass".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.username, "alice");
    }

    #[tokio::test]
    async fn get_user_not_found_returns_not_found() {
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let err = handler::get_user(
            &make_core_services(user_repo, MockApiKeyRepository::new()),
            GetUserRequest { username: "ghost".into() },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_users_returns_empty_list() {
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_list().returning(|_| Box::pin(async { Ok(vec![]) }));
        let resp = handler::list_users(&make_core_services(user_repo, MockApiKeyRepository::new()), ListUsersRequest {})
            .await
            .unwrap();
        assert_eq!(resp.users, []);
    }

    #[tokio::test]
    async fn delete_user_not_found_returns_not_found() {
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let err = handler::delete_user(
            &make_core_services(user_repo, MockApiKeyRepository::new()),
            DeleteUserRequest { username: "ghost".into() },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn update_user_not_found_returns_not_found() {
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_username().returning(|_, _| Box::pin(async { Ok(None) }));
        let err = handler::update_user(
            &make_core_services(user_repo, MockApiKeyRepository::new()),
            UpdateUserRequest {
                username: "ghost".into(),
                full_name: Some("Ghost".into()),
                email: None,
            },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }
}
