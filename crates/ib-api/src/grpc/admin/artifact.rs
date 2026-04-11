pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{
        CoreServices, Error, RepositoryError,
        artifact::model::{ArtifactKind, IssueArtifact, NewArtifact},
    };

    use crate::grpc::admin_proto::{
        AddArtifactRequest, ArtifactResponse, Empty, ListArtifactsRequest, ListArtifactsResponse, MoveArtifactRequest, RemoveArtifactRequest,
        UpdateArtifactRequest,
    };

    fn artifact_to_proto(artifact: IssueArtifact) -> ArtifactResponse {
        ArtifactResponse {
            slug: artifact.slug,
            kind: artifact.kind.to_string(),
            body_json: serde_json::to_string(&artifact.body).unwrap_or_default(),
            created_at: artifact.created_at.to_rfc3339(),
            updated_at: artifact.updated_at.to_rfc3339(),
        }
    }

    pub(crate) async fn add_artifact(core: &Arc<CoreServices>, req: AddArtifactRequest, created_by: String) -> Result<ArtifactResponse, Error> {
        let issue = core
            .issue_service()
            .find_by_slug(&req.issue_slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let kind: ArtifactKind = req
            .kind
            .parse()
            .map_err(|_| Error::Validation(format!("unknown artifact kind: {}", req.kind)))?;
        let body: serde_json::Value = serde_json::from_str(&req.body_json).map_err(|e| Error::Validation(format!("invalid body_json: {e}")))?;
        let new_artifact = NewArtifact {
            issue_id: issue.id,
            kind,
            slug: req.slug,
            body,
            created_by,
        };
        let artifact = core.artifact_service().add_artifact(new_artifact).await?;
        Ok(artifact_to_proto(artifact))
    }

    pub(crate) async fn update_artifact(core: &Arc<CoreServices>, req: UpdateArtifactRequest) -> Result<ArtifactResponse, Error> {
        let issue = core
            .issue_service()
            .find_by_slug(&req.issue_slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let body: serde_json::Value = serde_json::from_str(&req.body_json).map_err(|e| Error::Validation(format!("invalid body_json: {e}")))?;
        let artifact = core
            .artifact_service()
            // artifact_number is uint32 in proto; ArtifactId is u64 — widening cast, always safe
            .update_artifact_by_id(issue.id, req.artifact_number as u64, body)
            .await?;
        Ok(artifact_to_proto(artifact))
    }

    pub(crate) async fn remove_artifact(core: &Arc<CoreServices>, req: RemoveArtifactRequest) -> Result<(), Error> {
        let issue = core
            .issue_service()
            .find_by_slug(&req.issue_slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        // artifact_number is uint32 in proto; ArtifactId is u64 — widening cast, always
        // safe
        core.artifact_service().remove_artifact_by_id(issue.id, req.artifact_number as u64).await
    }

    pub(crate) async fn list_artifacts(core: &Arc<CoreServices>, req: ListArtifactsRequest) -> Result<ListArtifactsResponse, Error> {
        let issue = core
            .issue_service()
            .find_by_slug(&req.issue_slug)
            .await?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let kinds: Option<Vec<ArtifactKind>> = if req.kinds.is_empty() {
            None
        } else {
            Some(
                req.kinds
                    .iter()
                    .map(|k| k.parse().map_err(|_| Error::Validation(format!("unknown artifact kind: {k}"))))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        };
        let artifacts = core.artifact_service().list_artifacts(issue.id, kinds, false).await?;
        let responses = artifacts.into_iter().map(artifact_to_proto).collect();
        Ok(ListArtifactsResponse { artifacts: responses })
    }

    pub(crate) async fn move_artifact(core: &Arc<CoreServices>, req: MoveArtifactRequest) -> Result<Empty, Error> {
        core.artifact_service().move_artifact(&req.old_path, &req.new_path).await?;
        Ok(Empty {})
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
                AddArtifactRequest, ArtifactResponse, Empty, ListArtifactsRequest, ListArtifactsResponse, MoveArtifactRequest, RemoveArtifactRequest,
                UpdateArtifactRequest, admin_service_client::AdminServiceClient,
            },
        },
    };

    pub async fn add_artifact(host: &str, port: u16, issue_slug: &str, kind: &str, slug: Option<&str>, body_json: &str) -> Result<ArtifactResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(AddArtifactRequest {
            issue_slug: issue_slug.to_owned(),
            kind: kind.to_owned(),
            slug: slug.map(str::to_owned),
            body_json: body_json.to_owned(),
        }));
        client
            .add_artifact(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn update_artifact(host: &str, port: u16, issue_slug: &str, artifact_number: u32, body_json: &str) -> Result<ArtifactResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(UpdateArtifactRequest {
            issue_slug: issue_slug.to_owned(),
            artifact_number,
            body_json: body_json.to_owned(),
        }));
        client
            .update_artifact(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn remove_artifact(host: &str, port: u16, issue_slug: &str, artifact_number: u32) -> Result<(), Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(RemoveArtifactRequest {
            issue_slug: issue_slug.to_owned(),
            artifact_number,
        }));
        client
            .remove_artifact(req)
            .await
            .map(|_| ())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn list_artifacts(host: &str, port: u16, issue_slug: &str, kinds: Vec<String>) -> Result<ListArtifactsResponse, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(ListArtifactsRequest {
            issue_slug: issue_slug.to_owned(),
            kinds,
        }));
        client
            .list_artifacts(req)
            .await
            .map(|r| r.into_inner())
            .map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))
    }

    pub async fn move_artifact(host: &str, port: u16, old_path: &str, new_path: &str) -> Result<Empty, Error> {
        let mut client: AdminServiceClient<Channel> = make_client(host, port).await?;
        let req = with_api_key(tonic::Request::new(MoveArtifactRequest {
            old_path: old_path.to_owned(),
            new_path: new_path.to_owned(),
        }));
        client
            .move_artifact(req)
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
        artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        },
        issue::{IssuePriority, IssueStatus, IssueToken, MockIssueRepository},
        project::{MockProjectMemberRepository, MockProjectRepository},
        user::MockUserRepository,
    };
    use tonic::Code;

    use crate::grpc::{
        admin::{GrpcAdminService, artifact::handler},
        admin_proto::{AddArtifactRequest, ListArtifactsRequest, MoveArtifactRequest, RemoveArtifactRequest, UpdateArtifactRequest},
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

    fn no_member_repo() -> MockProjectMemberRepository {
        let mut r = MockProjectMemberRepository::new();
        r.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        r
    }

    fn make_service_non_member(issue_repo: MockIssueRepository, artifact_repo: MockArtifactRepository) -> GrpcAdminService {
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
                .project_repository(Arc::new(MockProjectRepository::new()))
                .project_member_repository(Arc::new(no_member_repo()))
                .issue_repository(Arc::new(issue_repo))
                .artifact_repository(Arc::new(artifact_repo))
                .build()
                .unwrap(),
        );
        GrpcAdminService::new(ib_core::create_services(repo_svc))
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
            submitter: 0,
            assigned: None,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn fake_artifact(id: u64, issue_id: u64, kind: ArtifactKind, slug: Option<String>, body: serde_json::Value) -> IssueArtifact {
        use chrono::Utc;
        IssueArtifact {
            id,
            token: ArtifactToken::new(id),
            issue_id,
            kind,
            slug,
            body,
            created_by: "U_1".to_owned(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_core(issue_repo: MockIssueRepository, artifact_repo: MockArtifactRepository) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(MockUserRepository::new()))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(MockProjectRepository::new()))
                .issue_repository(Arc::new(issue_repo))
                .artifact_repository(Arc::new(artifact_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    #[tokio::test]
    async fn add_artifact_success() {
        let issue = fake_issue(10, 1, 1);
        let artifact = fake_artifact(
            99,
            10,
            ArtifactKind::TriageResult,
            Some("triage".into()),
            serde_json::json!({"path": "insights/triage/t-1.md"}),
        );

        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let a = artifact.clone();
            artifact_repo.expect_create().returning(move |_, _| {
                let a = a.clone();
                Box::pin(async move { Ok(a) })
            });
        }

        let core = make_core(issue_repo, artifact_repo);
        let resp = handler::add_artifact(
            &core,
            AddArtifactRequest {
                issue_slug: "TP-1".into(),
                kind: "TriageResult".into(),
                slug: None,
                body_json: r#"{"path":"insights/triage/t-1.md"}"#.into(),
            },
            "U_1".into(),
        )
        .await
        .unwrap();
        assert_eq!(resp.kind, "TriageResult");
    }

    #[tokio::test]
    async fn add_artifact_issue_not_found() {
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));

        let core = make_core(issue_repo, MockArtifactRepository::new());
        let err = handler::add_artifact(
            &core,
            AddArtifactRequest {
                issue_slug: "TP-999".into(),
                kind: "Comment".into(),
                slug: Some("my-comment".into()),
                body_json: r#"{"text":"hello"}"#.into(),
            },
            "U_1".into(),
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn add_artifact_unknown_kind_returns_invalid_argument() {
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }

        let core = make_core(issue_repo, MockArtifactRepository::new());
        let err = handler::add_artifact(
            &core,
            AddArtifactRequest {
                issue_slug: "TP-1".into(),
                kind: "NotAKind".into(),
                slug: None,
                body_json: r#"{}"#.into(),
            },
            "U_1".into(),
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn list_artifacts_success() {
        let issue = fake_issue(10, 1, 1);
        let artifact = fake_artifact(1, 10, ArtifactKind::Comment, Some("my-comment".into()), serde_json::json!({"text": "hi"}));

        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let a = artifact.clone();
            artifact_repo.expect_list().returning(move |_, _, _| {
                let a = a.clone();
                Box::pin(async move { Ok(vec![a]) })
            });
        }

        let core = make_core(issue_repo, artifact_repo);
        let resp = handler::list_artifacts(
            &core,
            ListArtifactsRequest {
                issue_slug: "TP-1".into(),
                kinds: vec![],
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.artifacts.len(), 1);
        assert_eq!(resp.artifacts[0].kind, "Comment");
    }

    #[tokio::test]
    async fn remove_artifact_not_found_returns_not_found() {
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));

        let core = make_core(issue_repo, artifact_repo);
        let err = handler::remove_artifact(
            &core,
            RemoveArtifactRequest {
                issue_slug: "TP-1".into(),
                artifact_number: 999,
            },
        )
        .await
        .map_err(map_core_error)
        .unwrap_err();
        assert_eq!(err.code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn update_artifact_success() {
        let issue = fake_issue(10, 1, 1);
        let artifact_updated = fake_artifact(5, 10, ArtifactKind::Comment, Some("my-comment".into()), serde_json::json!({"text": "updated"}));

        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let found = fake_artifact(5, 10, ArtifactKind::Comment, Some("my-comment".into()), serde_json::json!({"text": "old"}));
            artifact_repo.expect_find_by_id().returning(move |_, _| {
                let f = found.clone();
                Box::pin(async move { Ok(Some(f)) })
            });
        }
        {
            let a = artifact_updated.clone();
            artifact_repo.expect_update().returning(move |_, _| {
                let a = a.clone();
                Box::pin(async move { Ok(a) })
            });
        }

        let core = make_core(issue_repo, artifact_repo);
        let resp = handler::update_artifact(
            &core,
            UpdateArtifactRequest {
                issue_slug: "TP-1".into(),
                artifact_number: 5,
                body_json: r#"{"text":"updated"}"#.into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.kind, "Comment");
    }

    #[tokio::test]
    async fn add_artifact_non_member_returns_not_found() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(issue_repo, MockArtifactRepository::new());
        let req = make_request_with_api_key(AddArtifactRequest {
            issue_slug: "TP-1".into(),
            kind: "Comment".into(),
            slug: None,
            body_json: r#"{"text":"hello"}"#.into(),
        });
        let err = svc.add_artifact(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn update_artifact_non_member_returns_not_found() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(issue_repo, MockArtifactRepository::new());
        let req = make_request_with_api_key(UpdateArtifactRequest {
            issue_slug: "TP-1".into(),
            artifact_number: 1,
            body_json: r#"{"text":"updated"}"#.into(),
        });
        let err = svc.update_artifact(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn remove_artifact_non_member_returns_not_found() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(issue_repo, MockArtifactRepository::new());
        let req = make_request_with_api_key(RemoveArtifactRequest {
            issue_slug: "TP-1".into(),
            artifact_number: 1,
        });
        let err = svc.remove_artifact(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_artifacts_non_member_returns_not_found() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(issue_repo, MockArtifactRepository::new());
        let req = make_request_with_api_key(ListArtifactsRequest {
            issue_slug: "TP-1".into(),
            kinds: vec![],
        });
        let err = svc.list_artifacts(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn move_artifact_non_admin_returns_permission_denied() {
        use crate::grpc::admin_proto::admin_service_server::AdminService;
        let svc = make_service_non_member(MockIssueRepository::new(), MockArtifactRepository::new());
        let req = make_request_with_api_key(MoveArtifactRequest {
            old_path: "old/path".into(),
            new_path: "new/path".into(),
        });
        let err = svc.move_artifact(req).await.unwrap_err();
        assert_eq!(err.code(), Code::PermissionDenied);
    }
}
