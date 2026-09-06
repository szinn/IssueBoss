pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        project::{NewProject, NewProjectMember},
        types::Capabilities,
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
        let new_project = NewProject::new(&req.name, &req.slug, &req.prefix, req.description)?;
        let project = core.project_service().create_project(new_project).await?;
        Ok(project_to_proto(project))
    }

    pub(crate) async fn update_project(core: &Arc<CoreServices>, req: UpdateProjectRequest) -> Result<ProjectResponse, Error> {
        let mut project = core
            .project_service()
            .find_by_slug(&req.slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        if let Some(name) = req.name {
            project.name = name;
        }
        let updated = core.project_service().update_project(project).await?;
        Ok(project_to_proto(updated))
    }

    pub(crate) async fn delete_project(core: &Arc<CoreServices>, req: DeleteProjectRequest) -> Result<Empty, Error> {
        let project = core
            .project_service()
            .find_by_slug(&req.slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        core.project_service().delete_project(project.id).await?;
        Ok(Empty {})
    }

    pub(crate) async fn get_project(core: &Arc<CoreServices>, req: GetProjectRequest) -> Result<ProjectResponse, Error> {
        let project = core
            .project_service()
            .find_by_slug(&req.slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        Ok(project_to_proto(project))
    }

    pub(crate) async fn list_projects(core: &Arc<CoreServices>, _req: ListProjectsRequest) -> Result<ListProjectsResponse, Error> {
        let projects = core.project_service().list_projects().await?;
        Ok(ListProjectsResponse {
            projects: projects.into_iter().map(project_to_proto).collect(),
        })
    }

    pub(crate) async fn add_project_member(core: &Arc<CoreServices>, req: AddProjectMemberRequest) -> Result<ProjectMemberResponse, Error> {
        let project = core
            .project_service()
            .find_by_slug(&req.project_slug)
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
        Ok(project_member_to_proto(&member, &user))
    }

    pub(crate) async fn update_project_member(core: &Arc<CoreServices>, req: UpdateProjectMemberRequest) -> Result<ProjectMemberResponse, Error> {
        let project = core
            .project_service()
            .find_by_slug(&req.project_slug)
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
        Ok(project_member_to_proto(&updated, &user))
    }

    pub(crate) async fn remove_project_member(core: &Arc<CoreServices>, req: RemoveProjectMemberRequest) -> Result<Empty, Error> {
        let project = core
            .project_service()
            .find_by_slug(&req.project_slug)
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
        let project = core
            .project_service()
            .find_by_slug(&req.project_slug)
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
            responses.push(project_member_to_proto(&member, &user));
        }
        Ok(ListProjectMembersResponse { members: responses })
    }

    fn parse_capabilities(raw: &[String]) -> Result<Capabilities, Error> {
        let caps: Result<Vec<ib_core::types::Capability>, _> = raw
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

    pub async fn create_project(host: &str, port: u16, name: &str, slug: &str, prefix: &str, description: Option<&str>) -> Result<ProjectResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(CreateProjectRequest {
            name: name.to_owned(),
            slug: slug.to_owned(),
            prefix: prefix.to_owned(),
            description: description.map(str::to_owned),
        }));
        client
            .create_project(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn get_project(host: &str, port: u16, slug: &str) -> Result<ProjectResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(GetProjectRequest { slug: slug.to_owned() }));
        client
            .get_project(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn list_projects(host: &str, port: u16) -> Result<ListProjectsResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListProjectsRequest {}));
        client
            .list_projects(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn update_project(host: &str, port: u16, slug: &str, name: Option<&str>) -> Result<ProjectResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(UpdateProjectRequest {
            slug: slug.to_owned(),
            name: name.map(str::to_owned),
        }));
        client
            .update_project(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn delete_project(host: &str, port: u16, slug: &str) -> Result<Empty, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(DeleteProjectRequest { slug: slug.to_owned() }));
        client
            .delete_project(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn add_project_member(
        host: &str,
        port: u16,
        project_slug: &str,
        username: &str,
        capabilities: Vec<String>,
    ) -> Result<ProjectMemberResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(AddProjectMemberRequest {
            project_slug: project_slug.to_owned(),
            username: username.to_owned(),
            capabilities,
        }));
        client
            .add_project_member(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn update_project_member(
        host: &str,
        port: u16,
        project_slug: &str,
        username: &str,
        capabilities: Vec<String>,
    ) -> Result<ProjectMemberResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(UpdateProjectMemberRequest {
            project_slug: project_slug.to_owned(),
            username: username.to_owned(),
            capabilities,
        }));
        client
            .update_project_member(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn remove_project_member(host: &str, port: u16, project_slug: &str, username: &str) -> Result<Empty, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(RemoveProjectMemberRequest {
            project_slug: project_slug.to_owned(),
            username: username.to_owned(),
        }));
        client
            .remove_project_member(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn list_project_members(host: &str, port: u16, project_slug: &str) -> Result<ListProjectMembersResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListProjectMembersRequest {
            project_slug: project_slug.to_owned(),
        }));
        client
            .list_project_members(req)
            .await
            .map(tonic::Response::into_inner)
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use ib_core::{
        api_key::MockApiKeyRepository,
        project::{MockProjectMemberRepository, MockProjectRepository, Project, ProjectMember, ProjectToken},
        types::Capabilities,
        user::{MockUserRepository, User, UserToken},
    };
    use tonic::Code;

    use crate::grpc::{
        admin::{project::handler, tests::make_core_services},
        admin_proto::{
            AddProjectMemberRequest, CreateProjectRequest, DeleteProjectRequest, GetProjectRequest, ListProjectMembersRequest, ListProjectsRequest,
            RemoveProjectMemberRequest, UpdateProjectMemberRequest, UpdateProjectRequest,
        },
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
            description: None,
            version: 0,
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
                description: None,
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
                description: None,
            },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(resp.code(), Code::InvalidArgument);
    }

    fn fake_user(id: u64, username: &str) -> User {
        User {
            id,
            token: UserToken::new(id),
            username: username.to_owned(),
            full_name: username.to_owned(),
            password_hash: String::new(),
            email_address: format!("{username}@example.com"),
            capabilities: Capabilities::default(),
            change_password_on_login: false,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn fake_member(project_id: u64, user_id: u64) -> ProjectMember {
        ProjectMember {
            project_id,
            user_id,
            capabilities: Capabilities::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn build_core(
        project_repo: MockProjectRepository,
        member_repo: MockProjectMemberRepository,
        user_repo: MockUserRepository,
    ) -> std::sync::Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(user_repo))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .project_member_repository(std::sync::Arc::new(member_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    #[tokio::test]
    async fn update_project_success() {
        let project = fake_project(1, "myapp");
        let updated = Project {
            name: "New Name".into(),
            ..project.clone()
        };
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            })
        };
        {
            let u = updated.clone();
            project_repo.expect_update().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(u) })
            })
        };
        let core = build_core(project_repo, MockProjectMemberRepository::new(), MockUserRepository::new());
        let resp = handler::update_project(
            &core,
            UpdateProjectRequest {
                slug: "myapp".into(),
                name: Some("New Name".into()),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.name, "New Name");
    }

    #[tokio::test]
    async fn delete_project_success() {
        let project = fake_project(1, "myapp");
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            })
        };
        {
            // delete_project service re-loads by id inside the transaction
            let p = project.clone();
            project_repo.expect_find_by_id().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            })
        };
        project_repo.expect_delete().returning(|_, p| Box::pin(async move { Ok(p) }));
        let core = build_core(project_repo, MockProjectMemberRepository::new(), MockUserRepository::new());
        handler::delete_project(&core, DeleteProjectRequest { slug: "myapp".into() }).await.unwrap();
    }

    #[tokio::test]
    async fn add_project_member_success() {
        let project = fake_project(1, "myapp");
        let user = fake_user(10, "alice");
        let member = fake_member(1, 10);
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            })
        };
        {
            // add_member service re-loads project by id inside the transaction
            let p = project.clone();
            project_repo.expect_find_by_id().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            })
        };
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_username().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            })
        };
        let mut member_repo = MockProjectMemberRepository::new();
        {
            let m = member.clone();
            member_repo.expect_create().returning(move |_, _| {
                let m = m.clone();
                Box::pin(async move { Ok(m) })
            })
        };
        let core = build_core(project_repo, member_repo, user_repo);
        let resp = handler::add_project_member(
            &core,
            AddProjectMemberRequest {
                project_slug: "myapp".into(),
                username: "alice".into(),
                capabilities: vec![],
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.username, "alice");
    }

    #[tokio::test]
    async fn update_project_member_success() {
        let project = fake_project(1, "myapp");
        let user = fake_user(10, "alice");
        let member = fake_member(1, 10);
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            })
        };
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_username().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            })
        };
        let mut member_repo = MockProjectMemberRepository::new();
        {
            let m = member.clone();
            member_repo.expect_find().returning(move |_, _, _| {
                let m = m.clone();
                Box::pin(async move { Ok(Some(m)) })
            })
        };
        {
            let m = member.clone();
            member_repo.expect_update().returning(move |_, _| {
                let m = m.clone();
                Box::pin(async move { Ok(m) })
            })
        };
        let core = build_core(project_repo, member_repo, user_repo);
        let resp = handler::update_project_member(
            &core,
            UpdateProjectMemberRequest {
                project_slug: "myapp".into(),
                username: "alice".into(),
                capabilities: vec![],
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.username, "alice");
    }

    #[tokio::test]
    async fn remove_project_member_success() {
        let project = fake_project(1, "myapp");
        let user = fake_user(10, "alice");
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            })
        };
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_username().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            })
        };
        let mut member_repo = MockProjectMemberRepository::new();
        {
            // remove_member service checks member exists before deleting
            let m = fake_member(1, 10);
            member_repo.expect_find().returning(move |_, _, _| {
                let m = m.clone();
                Box::pin(async move { Ok(Some(m)) })
            })
        };
        member_repo.expect_delete().returning(|_, _, _| Box::pin(async { Ok(()) }));
        let core = build_core(project_repo, member_repo, user_repo);
        handler::remove_project_member(
            &core,
            RemoveProjectMemberRequest {
                project_slug: "myapp".into(),
                username: "alice".into(),
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn list_project_members_success() {
        let project = fake_project(1, "myapp");
        let user = fake_user(10, "alice");
        let member = fake_member(1, 10);
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            })
        };
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            })
        };
        let mut member_repo = MockProjectMemberRepository::new();
        {
            let m = member.clone();
            member_repo.expect_list().returning(move |_, _| {
                let m = m.clone();
                Box::pin(async move { Ok(vec![m]) })
            })
        };
        let core = build_core(project_repo, member_repo, user_repo);
        let resp = handler::list_project_members(&core, ListProjectMembersRequest { project_slug: "myapp".into() })
            .await
            .unwrap();
        assert_eq!(resp.members.len(), 1);
        assert_eq!(resp.members[0].username, "alice");
    }
}
