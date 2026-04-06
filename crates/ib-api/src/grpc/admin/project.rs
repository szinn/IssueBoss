pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        project::{NewProject, NewProjectMember, ProjectToken},
        user::Capabilities,
    };

    use crate::grpc::{
        admin::{project_member_to_proto, project_to_proto},
        admin_proto::{
            AddProjectMemberRequest, CreateProjectRequest, DeleteProjectRequest, Empty, GetProjectRequest, ListProjectMembersRequest,
            ListProjectMembersResponse, ListProjectsRequest, ListProjectsResponse, ProjectMemberResponse, ProjectResponse, RemoveProjectMemberRequest,
            UpdateProjectMemberRequest, UpdateProjectRequest,
        },
    };

    pub(crate) async fn create_project(core: &Arc<CoreServices>, req: CreateProjectRequest) -> Result<ProjectResponse, Error> {
        let new_project = NewProject::new(&req.name, &req.slug, &req.prefix)?;
        let project = core.project_service().create_project(new_project).await?;
        Ok(project_to_proto(project))
    }

    pub async fn update_project(core: &Arc<CoreServices>, req: UpdateProjectRequest) -> Result<ProjectResponse, Error> {
        let token = ProjectToken::parse(&req.token).map_err(|_| Error::Validation(format!("invalid project token: {}", req.token)))?;
        let mut project = core
            .project_service()
            .find_by_token(token)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        if let Some(name) = req.name {
            project.name = name;
        }
        let updated = core.project_service().update_project(project).await?;
        Ok(project_to_proto(updated))
    }

    pub async fn delete_project(core: &Arc<CoreServices>, req: DeleteProjectRequest) -> Result<Empty, Error> {
        let token = ProjectToken::parse(&req.token).map_err(|_| Error::Validation(format!("invalid project token: {}", req.token)))?;
        let project = core
            .project_service()
            .find_by_token(token)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        core.project_service().delete_project(project.id).await?;
        Ok(Empty {})
    }

    pub async fn get_project(core: &Arc<CoreServices>, req: GetProjectRequest) -> Result<ProjectResponse, Error> {
        let project = core
            .project_service()
            .find_by_slug(&req.slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        Ok(project_to_proto(project))
    }

    pub async fn list_projects(core: &Arc<CoreServices>, _req: ListProjectsRequest) -> Result<ListProjectsResponse, Error> {
        let projects = core.project_service().list_projects().await?;
        Ok(ListProjectsResponse {
            projects: projects.into_iter().map(project_to_proto).collect(),
        })
    }

    pub async fn add_project_member(core: &Arc<CoreServices>, req: AddProjectMemberRequest) -> Result<ProjectMemberResponse, Error> {
        let project_token = ProjectToken::parse(&req.project_token).map_err(|_| Error::Validation(format!("invalid project token: {}", req.project_token)))?;
        let project = core
            .project_service()
            .find_by_token(project_token)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let user = core
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let capabilities = parse_capabilities(&req.capabilities)?;
        let member = core
            .project_service()
            .add_member(NewProjectMember {
                project_id: project.id,
                user_id: user.id,
                capabilities,
            })
            .await?;
        Ok(project_member_to_proto(member, &user))
    }

    pub async fn update_project_member(core: &Arc<CoreServices>, req: UpdateProjectMemberRequest) -> Result<ProjectMemberResponse, Error> {
        let project_token = ProjectToken::parse(&req.project_token).map_err(|_| Error::Validation(format!("invalid project token: {}", req.project_token)))?;
        let project = core
            .project_service()
            .find_by_token(project_token)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let user = core
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let mut member = core
            .project_service()
            .find_member(project.id, user.id)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        member.capabilities = parse_capabilities(&req.capabilities)?;
        let updated = core.project_service().update_member(member).await?;
        Ok(project_member_to_proto(updated, &user))
    }

    pub(crate) async fn remove_project_member(core: &Arc<CoreServices>, req: RemoveProjectMemberRequest) -> Result<Empty, Error> {
        let project_token = ProjectToken::parse(&req.project_token).map_err(|_| Error::Validation(format!("invalid project token: {}", req.project_token)))?;
        let project = core
            .project_service()
            .find_by_token(project_token)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let user = core
            .user_service()
            .find_by_username(&req.username)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        core.project_service().remove_member(project.id, user.id).await?;
        Ok(Empty {})
    }

    pub(crate) async fn list_project_members(core: &Arc<CoreServices>, req: ListProjectMembersRequest) -> Result<ListProjectMembersResponse, Error> {
        let project_token = ProjectToken::parse(&req.project_token).map_err(|_| Error::Validation(format!("invalid project token: {}", req.project_token)))?;
        let project = core
            .project_service()
            .find_by_token(project_token)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let members = core.project_service().list_members(project.id).await?;
        let mut responses = Vec::with_capacity(members.len());
        for member in members {
            let user = core
                .user_service()
                .find_by_id(member.user_id)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            responses.push(project_member_to_proto(member, &user));
        }
        Ok(ListProjectMembersResponse { members: responses })
    }

    fn parse_capabilities(raw: &[String]) -> Result<Capabilities, Error> {
        let caps: Result<Vec<ib_core::user::Capability>, _> = raw
            .iter()
            .map(|s| serde_json::from_value(serde_json::Value::String(s.clone())).map_err(|_| Error::Validation(format!("unknown capability: {s}"))))
            .collect();
        Ok(Capabilities(caps?))
    }
}

pub mod api {
    use ib_core::Error;
    use tonic::transport::Channel;

    use crate::{
        error::ApiError,
        grpc::{
            admin::api::{make_client, with_api_key},
            admin_proto::{
                AddProjectMemberRequest, CreateProjectRequest, DeleteProjectRequest, Empty, GetProjectRequest, ListProjectMembersRequest,
                ListProjectMembersResponse, ListProjectsRequest, ListProjectsResponse, ProjectMemberResponse, ProjectResponse, RemoveProjectMemberRequest,
                UpdateProjectMemberRequest, UpdateProjectRequest, admin_service_client::AdminServiceClient,
            },
        },
    };

    pub async fn create_project(host: &str, port: u16, name: &str, slug: &str, prefix: &str) -> Result<ProjectResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(CreateProjectRequest {
            name: name.to_owned(),
            slug: slug.to_owned(),
            prefix: prefix.to_owned(),
        }));
        client
            .create_project(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn get_project(host: &str, port: u16, slug: &str) -> Result<ProjectResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(GetProjectRequest { slug: slug.to_owned() }));
        client
            .get_project(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn list_projects(host: &str, port: u16) -> Result<ListProjectsResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListProjectsRequest {}));
        client
            .list_projects(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn update_project(host: &str, port: u16, token: &str, name: Option<&str>) -> Result<ProjectResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(UpdateProjectRequest {
            token: token.to_owned(),
            name: name.map(str::to_owned),
        }));
        client
            .update_project(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn delete_project(host: &str, port: u16, token: &str) -> Result<Empty, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(DeleteProjectRequest { token: token.to_owned() }));
        client
            .delete_project(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn add_project_member(
        host: &str,
        port: u16,
        project_token: &str,
        username: &str,
        capabilities: Vec<String>,
    ) -> Result<ProjectMemberResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(AddProjectMemberRequest {
            project_token: project_token.to_owned(),
            username: username.to_owned(),
            capabilities,
        }));
        client
            .add_project_member(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn update_project_member(
        host: &str,
        port: u16,
        project_token: &str,
        username: &str,
        capabilities: Vec<String>,
    ) -> Result<ProjectMemberResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(UpdateProjectMemberRequest {
            project_token: project_token.to_owned(),
            username: username.to_owned(),
            capabilities,
        }));
        client
            .update_project_member(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn remove_project_member(host: &str, port: u16, project_token: &str, username: &str) -> Result<Empty, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(RemoveProjectMemberRequest {
            project_token: project_token.to_owned(),
            username: username.to_owned(),
        }));
        client
            .remove_project_member(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn list_project_members(host: &str, port: u16, project_token: &str) -> Result<ListProjectMembersResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListProjectMembersRequest {
            project_token: project_token.to_owned(),
        }));
        client
            .list_project_members(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use ib_core::{
        api_key::MockApiKeyRepository,
        project::{MockProjectRepository, Project, ProjectToken},
        user::MockUserRepository,
    };
    use tonic::Code;

    use crate::grpc::{
        admin::{project::handler, tests::make_core_services},
        admin_proto::{CreateProjectRequest, GetProjectRequest, ListProjectsRequest},
        error::map_core_error,
    };

    fn fake_project(id: u64, slug: &str) -> Project {
        Project {
            id,
            token: ProjectToken::new(id),
            name: format!("Project {slug}"),
            slug: slug.to_owned(),
            prefix: "TP".to_owned(),
            issue_counter: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_project_success() {
        let project = fake_project(1, "myapp");
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_create().returning(move |_, _| {
            let p = project.clone();
            Box::pin(async move { Ok(p) })
        });
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(MockUserRepository::new()))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        let resp = handler::create_project(
            &core,
            CreateProjectRequest {
                name: "MyApp".into(),
                slug: "myapp".into(),
                prefix: "TP".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.slug, "myapp");
    }

    #[tokio::test]
    async fn get_project_not_found_returns_not_found() {
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(MockUserRepository::new()))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        let err = handler::get_project(&core, GetProjectRequest { slug: "ghost".into() })
            .await
            .map_err(map_core_error)
            .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_projects_returns_all() {
        let projects = vec![fake_project(1, "app1"), fake_project(2, "app2")];
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_list().returning(move |_| {
            let p = projects.clone();
            Box::pin(async move { Ok(p) })
        });
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(MockUserRepository::new()))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        let resp = handler::list_projects(&core, ListProjectsRequest {}).await.unwrap();
        assert_eq!(resp.projects.len(), 2);
    }

    #[tokio::test]
    async fn create_project_invalid_prefix_returns_validation_error() {
        let resp = handler::create_project(
            &make_core_services(MockUserRepository::new(), MockApiKeyRepository::new()),
            CreateProjectRequest {
                name: "MyApp".into(),
                slug: "myapp".into(),
                prefix: "toolong".into(),
            },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(resp.code(), Code::InvalidArgument);
    }
}
