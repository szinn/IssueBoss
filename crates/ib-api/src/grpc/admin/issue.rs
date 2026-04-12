pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        issue::{IssueFilter, NewIssue},
        user::UserId,
    };

    use crate::grpc::{
        admin::issue_to_proto,
        admin_proto::{CreateIssueRequest, GetIssueRequest, IssueResponse, ListIssuesRequest, ListIssuesResponse, TransitionIssueRequest, UpdateIssueRequest},
    };

    pub(crate) async fn create_issue(core: &Arc<CoreServices>, req: CreateIssueRequest, submitted_by: UserId) -> Result<IssueResponse, Error> {
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
        let new_issue = NewIssue::new(project.id, &project.prefix, req.title, req.description, priority, size, submitted_by)?;
        let issue = core.issue_service().create_issue(new_issue).await?;
        issue_to_proto(core, issue, &project.slug).await
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
        issue_to_proto(core, updated, &project_slug).await
    }

    pub(crate) async fn transition_issue(core: &Arc<CoreServices>, req: TransitionIssueRequest, triggered_by: UserId) -> Result<IssueResponse, Error> {
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
        let updated = core.issue_service().transition_issue(issue.token, new_status, None, Some(triggered_by)).await?;
        issue_to_proto(core, updated, &project_slug).await
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
        issue_to_proto(core, issue, &project_slug).await
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
            exclude_blocked: req.exclude_blocked,
            submitted_by: None,
            assigned_to: None,
        };
        let issues = core.issue_service().list_issues(project.id, filter).await?;
        let mut responses = Vec::with_capacity(issues.len());
        for i in issues {
            responses.push(issue_to_proto(core, i, &project.slug).await?);
        }
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

    #[allow(clippy::too_many_arguments)]
    pub async fn list_issues(
        host: &str,
        port: u16,
        project_slug: &str,
        status: Option<&str>,
        priority: Option<&str>,
        size: Option<&str>,
        limit: Option<u64>,
        exclude_blocked: Option<bool>,
    ) -> Result<ListIssuesResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListIssuesRequest {
            project_slug: project_slug.to_owned(),
            status: status.map(str::to_owned),
            priority: priority.map(str::to_owned),
            size: size.map(str::to_owned),
            limit,
            exclude_blocked,
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
    use std::sync::Arc;

    use ib_core::{
        api_key::{ApiKey, MockApiKeyRepository, sha256_hex},
        issue::{IssuePriority, IssueStatus, IssueToken, MockIssueRepository},
        project::{MockProjectMemberRepository, MockProjectRepository, Project, ProjectToken},
        relationship::{IssueRelationships, MockIssueRelationshipRepository},
        user::MockUserRepository,
    };
    use tonic::Code;

    use crate::grpc::{
        admin::{GrpcAdminService, issue::handler},
        admin_proto::{CreateIssueRequest, GetIssueRequest, ListIssuesRequest, TransitionIssueRequest, UpdateIssueRequest},
        error::map_core_error,
    };

    const TEST_API_KEY: &str = "test-api-key-nonmember";

    fn fake_non_admin_user(id: u64) -> ib_core::user::User {
        use chrono::Utc;
        ib_core::user::User {
            id,
            token: ib_core::user::UserToken::new(id),
            username: "non_admin".to_owned(),
            full_name: "Non Admin".to_owned(),
            password_hash: "hash".to_owned(),
            email_address: "non_admin@example.com".to_owned(),
            capabilities: ib_core::types::Capabilities::default(),
            change_password_on_login: false,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn fake_api_key_for_user(user_id: u64) -> ApiKey {
        use chrono::Utc;
        ApiKey {
            id: 1,
            user_id,
            key_type: "ib_live".to_owned(),
            key_hash: sha256_hex(TEST_API_KEY),
            key_prefix: "ib_live_XXXX".to_owned(),
            name: None,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    fn make_request_with_api_key<T>(body: T) -> tonic::Request<T> {
        let mut req = tonic::Request::new(body);
        req.metadata_mut().insert("x-api-key", TEST_API_KEY.parse().unwrap());
        req
    }

    fn make_service_non_member(
        project_repo: MockProjectRepository,
        issue_repo: MockIssueRepository,
        member_repo: MockProjectMemberRepository,
    ) -> GrpcAdminService {
        let user = fake_non_admin_user(42);
        let api_key = fake_api_key_for_user(42);
        let key_hash = api_key.key_hash.clone();

        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }

        let mut api_key_repo = MockApiKeyRepository::new();
        {
            let k = api_key.clone();
            api_key_repo.expect_find_by_hash().returning(move |_, h| {
                if h == key_hash {
                    let k = k.clone();
                    Box::pin(async move { Ok(Some(k)) })
                } else {
                    Box::pin(async { Ok(None) })
                }
            });
        }
        api_key_repo.expect_update_last_used().returning(|_, _| Box::pin(async { Ok(()) }));

        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(user_repo))
                .api_key_repository(Arc::new(api_key_repo))
                .project_repository(Arc::new(project_repo))
                .project_member_repository(Arc::new(member_repo))
                .issue_repository(Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        GrpcAdminService::new(ib_core::create_services(repo_svc))
    }

    fn no_member_repo() -> MockProjectMemberRepository {
        let mut r = MockProjectMemberRepository::new();
        r.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        r
    }

    fn empty_relationship_repo() -> MockIssueRelationshipRepository {
        let mut r = MockIssueRelationshipRepository::new();
        r.expect_list_for_issue()
            .returning(|_, _| Box::pin(async { Ok(IssueRelationships::default()) }));
        r
    }

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
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("TP-{number}"),
            version: 0,
            submitter: 1,
            assigned: None,
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
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(user_repo))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .issue_repository(std::sync::Arc::new(issue_repo))
                .relationship_repository(std::sync::Arc::new(empty_relationship_repo()))
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
            1,
        )
        .await
        .unwrap();
        assert_eq!(resp.project_slug, "myapp");
        assert_eq!(resp.number, 1);
        assert_eq!(resp.submitter, "unknown");
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
        let issue_before = fake_issue(100, 1, 1); // status: TriageNeeded
        let issue_after = {
            let mut i = issue_before.clone();
            i.status = IssueStatus::TriageInProgress;
            i
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
                .withf(|_, issue| issue.status == IssueStatus::TriageInProgress)
                .returning(move |_, _| {
                    let i = i.clone();
                    Box::pin(async move { Ok(i) })
                });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));
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
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(user_repo))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .issue_repository(std::sync::Arc::new(issue_repo))
                .artifact_repository(std::sync::Arc::new(artifact_repo))
                .relationship_repository(std::sync::Arc::new(empty_relationship_repo()))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        let resp = handler::transition_issue(
            &core,
            crate::grpc::admin_proto::TransitionIssueRequest {
                slug: "MA-1".into(),
                new_status: "TriageInProgress".into(),
            },
            1,
        )
        .await
        .unwrap();
        assert_eq!(resp.status, "TriageInProgress");
    }

    #[tokio::test]
    async fn transition_issue_illegal_transition_returns_invalid_argument() {
        let project = fake_project(1, "myapp", "MA");
        let issue = fake_issue(100, 1, 1); // status: TriageNeeded
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
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(user_repo))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .issue_repository(std::sync::Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);
        // TriageNeeded → DevReview is an illegal skip (cannot jump categories)
        let err = handler::transition_issue(
            &core,
            crate::grpc::admin_proto::TransitionIssueRequest {
                slug: "MA-1".into(),
                new_status: "DevReview".into(),
            },
            1,
        )
        .await
        .map_err(crate::grpc::error::map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn transition_issue_unknown_status_returns_invalid_argument() {
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(user_repo))
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
            1,
        )
        .await
        .map_err(crate::grpc::error::map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn create_issue_passes_submitted_by() {
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
            issue_repo
                .expect_create()
                // submitted_by should be 42 (our test user id)
                .withf(|_, r| r.submitted_by == 42)
                .returning(move |_, _| {
                    let i = i.clone();
                    Box::pin(async move { Ok(i) })
                });
        }
        let mut user_repo = MockUserRepository::new();
        user_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(user_repo))
                .api_key_repository(std::sync::Arc::new(MockApiKeyRepository::new()))
                .project_repository(std::sync::Arc::new(project_repo))
                .issue_repository(std::sync::Arc::new(issue_repo))
                .relationship_repository(std::sync::Arc::new(empty_relationship_repo()))
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
            42,
        )
        .await
        .unwrap();
        assert_eq!(resp.project_slug, "myapp");
        assert_eq!(resp.submitter, "unknown"); // user 42 not in repo → "unknown"
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
            1,
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), Code::InvalidArgument);
    }

    #[tokio::test]
    async fn list_issues_non_member_returns_permission_error() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let project = fake_project(1, "myapp", "MA");
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let svc = make_service_non_member(project_repo, MockIssueRepository::new(), no_member_repo());
        let req = make_request_with_api_key(ListIssuesRequest {
            project_slug: "myapp".into(),
            status: None,
            priority: None,
            size: None,
            limit: None,
            exclude_blocked: None,
        });
        let err = svc.list_issues(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn create_issue_non_member_returns_permission_error() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let project = fake_project(1, "myapp", "MA");
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let svc = make_service_non_member(project_repo, MockIssueRepository::new(), no_member_repo());
        let req = make_request_with_api_key(CreateIssueRequest {
            project_slug: "myapp".into(),
            title: "New issue".into(),
            description: "".into(),
            priority: "Medium".into(),
            size: None,
        });
        let err = svc.create_issue(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn get_issue_non_member_returns_permission_error() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let issue = fake_issue(100, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(MockProjectRepository::new(), issue_repo, no_member_repo());
        let req = make_request_with_api_key(GetIssueRequest { slug: "TP-1".into() });
        let err = svc.get_issue(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn update_issue_non_member_returns_permission_error() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let issue = fake_issue(100, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(MockProjectRepository::new(), issue_repo, no_member_repo());
        let req = make_request_with_api_key(UpdateIssueRequest {
            slug: "TP-1".into(),
            title: Some("Updated title".into()),
            description: None,
            priority: None,
            size: None,
        });
        let err = svc.update_issue(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn transition_issue_non_member_returns_permission_error() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let issue = fake_issue(100, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(MockProjectRepository::new(), issue_repo, no_member_repo());
        let req = make_request_with_api_key(TransitionIssueRequest {
            slug: "TP-1".into(),
            new_status: "TriageInProgress".into(),
        });
        let err = svc.transition_issue(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }
}
