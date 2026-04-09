pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        issue::{IssueFilter, NewIssue},
    };

    use crate::grpc::{
        admin::issue_to_proto,
        admin_proto::{CreateIssueRequest, GetIssueRequest, IssueResponse, ListIssuesRequest, ListIssuesResponse, TransitionIssueRequest, UpdateIssueRequest},
    };

    pub(crate) async fn create_issue(core: &Arc<CoreServices>, req: CreateIssueRequest) -> Result<IssueResponse, Error> {
        let project = core
            .project_service()
            .find_by_slug(&req.project_slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let priority = req
            .priority
            .parse()
            .map_err(|_| Error::Validation(format!("unknown priority: {}", req.priority)))?;
        let size = req
            .size
            .map(|s| s.parse().map_err(|_| Error::Validation(format!("unknown size: {s}"))))
            .transpose()?;
        let new_issue = NewIssue::new(project.id, &project.prefix, req.title, req.description, priority, size)?;
        let issue = core.issue_service().create_issue(new_issue).await?;
        Ok(issue_to_proto(issue, &project.slug))
    }

    pub(crate) async fn update_issue(core: &Arc<CoreServices>, req: UpdateIssueRequest) -> Result<IssueResponse, Error> {
        let mut issue = core
            .issue_service()
            .find_by_slug(&req.slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let project_slug = core
            .project_service()
            .find_by_id(issue.project_id)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?
            .slug;
        if let Some(title) = req.title {
            issue.title = title;
        }
        if let Some(description) = req.description {
            issue.description = description;
        }
        if let Some(priority) = req.priority {
            issue.priority = priority.parse().map_err(|_| Error::Validation(format!("unknown priority: {priority}")))?;
        }
        if let Some(size) = req.size {
            issue.size = Some(size.parse().map_err(|_| Error::Validation(format!("unknown size: {size}")))?);
        }
        let updated = core.issue_service().update_issue(issue).await?;
        Ok(issue_to_proto(updated, &project_slug))
    }

    pub(crate) async fn transition_issue(core: &Arc<CoreServices>, req: TransitionIssueRequest) -> Result<IssueResponse, Error> {
        let new_status: ib_core::issue::IssueStatus = req
            .new_status
            .parse()
            .map_err(|_| Error::Validation(format!("unknown status: {}", req.new_status)))?;
        let issue = core
            .issue_service()
            .find_by_slug(&req.slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let project_slug = core
            .project_service()
            .find_by_id(issue.project_id)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?
            .slug;
        let updated = core.issue_service().transition_issue(issue.token, new_status, None).await?;
        Ok(issue_to_proto(updated, &project_slug))
    }

    pub(crate) async fn get_issue(core: &Arc<CoreServices>, req: GetIssueRequest) -> Result<IssueResponse, Error> {
        let issue = core
            .issue_service()
            .find_by_slug(&req.slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let project_slug = core
            .project_service()
            .find_by_id(issue.project_id)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?
            .slug;
        Ok(issue_to_proto(issue, &project_slug))
    }

    pub(crate) async fn list_issues(core: &Arc<CoreServices>, req: ListIssuesRequest) -> Result<ListIssuesResponse, Error> {
        let project = core
            .project_service()
            .find_by_slug(&req.project_slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let filter = IssueFilter {
            status: req
                .status
                .map(|s| s.parse().map_err(|_| Error::Validation(format!("unknown status: {s}"))))
                .transpose()?,
            priority: req
                .priority
                .map(|s| s.parse().map_err(|_| Error::Validation(format!("unknown priority: {s}"))))
                .transpose()?,
            size: req
                .size
                .map(|s| s.parse().map_err(|_| Error::Validation(format!("unknown size: {s}"))))
                .transpose()?,
            limit: req.limit,
        };
        let issues = core.issue_service().list_issues(project.id, filter).await?;
        let responses = issues.into_iter().map(|i| issue_to_proto(i, &project.slug)).collect();
        Ok(ListIssuesResponse { issues: responses })
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
                CreateIssueRequest, GetIssueRequest, IssueResponse, ListIssuesRequest, ListIssuesResponse, TransitionIssueRequest, UpdateIssueRequest,
                admin_service_client::AdminServiceClient,
            },
        },
    };

    pub async fn create_issue(
        host: &str,
        port: u16,
        project_slug: &str,
        title: &str,
        description: &str,
        priority: &str,
        size: Option<&str>,
    ) -> Result<IssueResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(CreateIssueRequest {
            project_slug: project_slug.to_owned(),
            title: title.to_owned(),
            description: description.to_owned(),
            priority: priority.to_owned(),
            size: size.map(str::to_owned),
        }));
        client
            .create_issue(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn get_issue(host: &str, port: u16, slug: &str) -> Result<IssueResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(GetIssueRequest { slug: slug.to_owned() }));
        client
            .get_issue(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_issue(
        host: &str,
        port: u16,
        slug: &str,
        title: Option<&str>,
        description: Option<&str>,
        priority: Option<&str>,
        size: Option<&str>,
    ) -> Result<IssueResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(UpdateIssueRequest {
            slug: slug.to_owned(),
            title: title.map(str::to_owned),
            description: description.map(str::to_owned),
            priority: priority.map(str::to_owned),
            size: size.map(str::to_owned),
        }));
        client
            .update_issue(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn list_issues(
        host: &str,
        port: u16,
        project_slug: &str,
        status: Option<&str>,
        priority: Option<&str>,
        size: Option<&str>,
        limit: Option<u64>,
    ) -> Result<ListIssuesResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListIssuesRequest {
            project_slug: project_slug.to_owned(),
            status: status.map(str::to_owned),
            priority: priority.map(str::to_owned),
            size: size.map(str::to_owned),
            limit,
        }));
        client
            .list_issues(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn transition_issue(host: &str, port: u16, slug: &str, new_status: &str) -> Result<IssueResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(TransitionIssueRequest {
            slug: slug.to_owned(),
            new_status: new_status.to_owned(),
        }));
        client
            .transition_issue(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use ib_core::{
        api_key::MockApiKeyRepository,
        issue::{IssuePriority, IssueStatus, IssueToken, MockIssueRepository},
        project::{MockProjectRepository, Project, ProjectToken},
        user::MockUserRepository,
    };
    use tonic::Code;

    use crate::grpc::{
        admin::issue::handler,
        admin_proto::{CreateIssueRequest, GetIssueRequest},
        error::map_core_error,
    };

    fn fake_project(id: u64, slug: &str, prefix: &str) -> Project {
        use chrono::Utc;
        Project {
            id,
            token: ProjectToken::new(id),
            name: format!("Project {slug}"),
            slug: slug.to_owned(),
            prefix: prefix.to_owned(),
            issue_counter: 0,
            description: None,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn fake_issue(id: u64, project_id: u64, number: u32) -> ib_core::issue::Issue {
        use chrono::Utc;
        ib_core::issue::Issue {
            id,
            token: IssueToken::new(id),
            number,
            project_id,
            title: format!("Issue {number}"),
            description: "".into(),
            status: IssueStatus::Triage,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("TP-{number}"),
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn create_issue_success() {
        let project = fake_project(1, "myapp", "MA");
        let issue = fake_issue(100, 1, 1);
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        {
            let p = project.clone();
            project_repo.expect_find_by_id().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        project_repo.expect_increment_issue_counter().returning(|_, _| Box::pin(async { Ok(1u32) }));
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_create().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(i) })
            });
        }
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(MockUserRepository::new()))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .issue_repository(std::sync::Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        let resp = handler::create_issue(
            &core,
            CreateIssueRequest {
                project_slug: "myapp".into(),
                title: "Fix login".into(),
                description: "".into(),
                priority: "High".into(),
                size: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.project_slug, "myapp");
        assert_eq!(resp.number, 1);
    }

    #[tokio::test]
    async fn get_issue_not_found_returns_not_found() {
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(MockUserRepository::new()))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .issue_repository(std::sync::Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        let err = handler::get_issue(&core, GetIssueRequest { slug: "TP-999".into() })
            .await
            .map_err(map_core_error)
            .unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn transition_issue_success() {
        use ib_core::artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        };
        let project = fake_project(1, "myapp", "MA");
        let issue_before = fake_issue(100, 1, 1); // status: Triage
        let issue_after = {
            let mut i = issue_before.clone();
            i.status = IssueStatus::SpecNeeded;
            i
        };
        let triage_artifact = IssueArtifact {
            id: 1,
            token: ArtifactToken::new(1),
            issue_id: 100,
            kind: ArtifactKind::TriageResult,
            slug: None,
            body: serde_json::json!({"path": "insights/triage/tp-1.md"}),
            created_by: "U_test".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        {
            let p = project.clone();
            project_repo.expect_find_by_id().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue_before.clone();
            // find_by_slug called by handler
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let i = issue_before.clone();
            // find_by_id called by transition_issue service (loads by token.id())
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let i = issue_after.clone();
            issue_repo
                .expect_update()
                .withf(|_, issue| issue.status == IssueStatus::SpecNeeded)
                .returning(move |_, _| {
                    let i = i.clone();
                    Box::pin(async move { Ok(i) })
                });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let art = triage_artifact.clone();
            artifact_repo.expect_list().returning(move |_, _, _| {
                let a = art.clone();
                Box::pin(async move { Ok(vec![a]) })
            });
        }
        artifact_repo.expect_create().returning(|_, _| {
            Box::pin(async move {
                Ok(IssueArtifact {
                    id: 999,
                    token: ArtifactToken::new(999),
                    issue_id: 100,
                    kind: ArtifactKind::StatusTransition,
                    slug: None,
                    body: serde_json::json!({}),
                    created_by: "system".into(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            })
        });
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(MockUserRepository::new()))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .issue_repository(std::sync::Arc::new(issue_repo))
                .artifact_repository(std::sync::Arc::new(artifact_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        let resp = handler::transition_issue(
            &core,
            crate::grpc::admin_proto::TransitionIssueRequest {
                slug: "MA-1".into(),
                new_status: "SpecNeeded".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.status, "SpecNeeded");
    }

    #[tokio::test]
    async fn transition_issue_illegal_transition_returns_invalid_argument() {
        let project = fake_project(1, "myapp", "MA");
        let issue = fake_issue(100, 1, 1); // status: Triage
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_id().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(MockUserRepository::new()))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .issue_repository(std::sync::Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        // Triage → Done is an illegal skip
        let err = handler::transition_issue(
            &core,
            crate::grpc::admin_proto::TransitionIssueRequest {
                slug: "MA-1".into(),
                new_status: "Done".into(),
            },
        )
        .await
        .map_err(crate::grpc::error::map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn transition_issue_unknown_status_returns_invalid_argument() {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(MockUserRepository::new()))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        let err = handler::transition_issue(
            &core,
            crate::grpc::admin_proto::TransitionIssueRequest {
                slug: "MA-1".into(),
                new_status: "NotARealStatus".into(),
            },
        )
        .await
        .map_err(crate::grpc::error::map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn create_issue_invalid_priority_returns_validation_error() {
        let project = fake_project(1, "myapp", "MA");
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_slug().returning(move |_, _| {
            let p = project.clone();
            Box::pin(async move { Ok(Some(p)) })
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
        let err = handler::create_issue(
            &core,
            CreateIssueRequest {
                project_slug: "myapp".into(),
                title: "Test".into(),
                description: "".into(),
                priority: "InvalidPriority".into(),
                size: None,
            },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }
}
