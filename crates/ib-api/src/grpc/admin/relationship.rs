pub(crate) mod handler {
    use std::sync::Arc;

    use ib_core::{CoreServices, Error, relationship::RelationshipKind};

    use crate::grpc::admin_proto::{
        AddRelationshipRequest, IssueRelationshipsProto, ListRelationshipsResponse, RelatedIssueSummary as ProtoRelatedIssueSummary, RelationshipResponse,
        RemoveRelationshipRequest,
    };

    pub(crate) fn relationships_to_proto(rels: ib_core::relationship::IssueRelationships) -> IssueRelationshipsProto {
        let convert = |v: Vec<ib_core::relationship::RelatedIssueSummary>| {
            v.into_iter()
                .map(|s| ProtoRelatedIssueSummary {
                    id: s.id as i64,
                    slug: s.slug,
                    title: s.title,
                })
                .collect()
        };
        IssueRelationshipsProto {
            depends_on: convert(rels.depends_on),
            blocks: convert(rels.blocks),
            related_to: convert(rels.related_to),
        }
    }

    pub(crate) async fn relationships_for_issue(core: &Arc<CoreServices>, issue_id: ib_core::issue::IssueId) -> Result<IssueRelationshipsProto, Error> {
        let rels = core.relationship_service().list_for_issue(issue_id).await?;
        Ok(relationships_to_proto(rels))
    }

    pub(crate) async fn add_relationship(core: &Arc<CoreServices>, req: AddRelationshipRequest) -> Result<RelationshipResponse, Error> {
        let kind = req
            .kind
            .parse::<RelationshipKind>()
            .map_err(|_| Error::Validation(format!("unknown relationship kind: {}", req.kind)))?;
        core.relationship_service().add_relationship(&req.from_slug, &req.to_slug, kind).await?;
        Ok(RelationshipResponse {
            from_slug: req.from_slug,
            to_slug: req.to_slug,
            kind: req.kind,
        })
    }

    pub(crate) async fn remove_relationship(core: &Arc<CoreServices>, req: RemoveRelationshipRequest) -> Result<bool, Error> {
        let kind = req
            .kind
            .parse::<RelationshipKind>()
            .map_err(|_| Error::Validation(format!("unknown relationship kind: {}", req.kind)))?;
        core.relationship_service().remove_relationship(&req.from_slug, &req.to_slug, kind).await
    }

    pub(crate) async fn list_relationships(core: &Arc<CoreServices>, issue_id: ib_core::issue::IssueId) -> Result<ListRelationshipsResponse, Error> {
        let rels = core.relationship_service().list_for_issue(issue_id).await?;
        Ok(ListRelationshipsResponse {
            relationships: Some(relationships_to_proto(rels)),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{
        api_key::{ApiKey, MockApiKeyRepository, sha256_hex},
        issue::{IssuePriority, IssueStatus, IssueToken, MockIssueRepository},
        project::{MockProjectMemberRepository, MockProjectRepository},
        relationship::{IssueRelationships, MockIssueRelationshipRepository, RelatedIssueSummary, model::IssueRelationship},
        user::MockUserRepository,
    };
    use tonic::Code;

    use crate::grpc::{
        admin::{GrpcAdminService, relationship::handler},
        admin_proto::{AddRelationshipRequest, ListRelationshipsRequest, RemoveRelationshipRequest, admin_service_server::AdminService},
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_core(issue_repo: MockIssueRepository, rel_repo: MockIssueRelationshipRepository) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(MockUserRepository::new()))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(MockProjectRepository::new()))
                .issue_repository(Arc::new(issue_repo))
                .relationship_repository(Arc::new(rel_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    fn make_service_non_member(issue_repo: MockIssueRepository, rel_repo: MockIssueRelationshipRepository) -> GrpcAdminService {
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
                .relationship_repository(Arc::new(rel_repo))
                .build()
                .unwrap(),
        );
        GrpcAdminService::new(ib_core::create_services(repo_svc))
    }

    // ── handler-level happy-path tests ───────────────────────────────────────

    #[tokio::test]
    async fn add_relationship_success() {
        let from_issue = fake_issue(10, 1, 1);
        let to_issue = fake_issue(11, 1, 2);

        let mut issue_repo = MockIssueRepository::new();
        {
            let from = from_issue.clone();
            let to = to_issue.clone();
            let mut call = 0u8;
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                call += 1;
                let issue = if call == 1 { from.clone() } else { to.clone() };
                Box::pin(async move { Ok(Some(issue)) })
            });
        }

        let mut rel_repo = MockIssueRelationshipRepository::new();
        {
            rel_repo
                .expect_list_for_issue()
                .returning(|_, _| Box::pin(async { Ok(IssueRelationships::default()) }));
            rel_repo.expect_add().returning(|_, record| {
                let rel = IssueRelationship {
                    id: 1,
                    from_issue_id: record.from_issue_id,
                    to_issue_id: record.to_issue_id,
                    kind: record.kind,
                    created_at: chrono::Utc::now(),
                };
                Box::pin(async move { Ok(rel) })
            });
        }

        let core = make_core(issue_repo, rel_repo);
        let resp = handler::add_relationship(
            &core,
            AddRelationshipRequest {
                from_slug: "TP-1".into(),
                to_slug: "TP-2".into(),
                kind: "DependsOn".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.from_slug, "TP-1");
        assert_eq!(resp.to_slug, "TP-2");
        assert_eq!(resp.kind, "DependsOn");
    }

    #[tokio::test]
    async fn remove_relationship_success() {
        let from_issue = fake_issue(10, 1, 1);
        let to_issue = fake_issue(11, 1, 2);

        let mut issue_repo = MockIssueRepository::new();
        {
            let from = from_issue.clone();
            let to = to_issue.clone();
            let mut call = 0u8;
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                call += 1;
                let issue = if call == 1 { from.clone() } else { to.clone() };
                Box::pin(async move { Ok(Some(issue)) })
            });
        }

        let mut rel_repo = MockIssueRelationshipRepository::new();
        rel_repo.expect_remove().returning(|_, _, _, _| Box::pin(async { Ok(true) }));

        let core = make_core(issue_repo, rel_repo);
        let removed = handler::remove_relationship(
            &core,
            RemoveRelationshipRequest {
                from_slug: "TP-1".into(),
                to_slug: "TP-2".into(),
                kind: "DependsOn".into(),
            },
        )
        .await
        .unwrap();
        assert!(removed);
    }

    #[tokio::test]
    async fn list_relationships_success() {
        let mut rel_repo = MockIssueRelationshipRepository::new();
        rel_repo.expect_list_for_issue().returning(|_, _| {
            Box::pin(async {
                Ok(IssueRelationships {
                    depends_on: vec![RelatedIssueSummary {
                        id: 11,
                        slug: "TP-2".into(),
                        title: "Issue 2".into(),
                    }],
                    blocks: vec![],
                    related_to: vec![],
                })
            })
        });

        let core = make_core(MockIssueRepository::new(), rel_repo);
        let resp = handler::list_relationships(&core, 10).await.unwrap();
        let rels = resp.relationships.expect("should have relationships");
        assert_eq!(rels.depends_on.len(), 1);
        assert_eq!(rels.depends_on[0].slug, "TP-2");
        assert!(rels.blocks.is_empty());
        assert!(rels.related_to.is_empty());
    }

    // ── permission / capability tests via full gRPC service ─────────────────

    #[tokio::test]
    async fn add_relationship_non_member_returns_not_found() {
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(issue_repo, MockIssueRelationshipRepository::new());
        let req = make_request_with_api_key(AddRelationshipRequest {
            from_slug: "TP-1".into(),
            to_slug: "TP-2".into(),
            kind: "DependsOn".into(),
        });
        let err = svc.add_relationship(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn remove_relationship_non_member_returns_not_found() {
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(issue_repo, MockIssueRelationshipRepository::new());
        let req = make_request_with_api_key(RemoveRelationshipRequest {
            from_slug: "TP-1".into(),
            to_slug: "TP-2".into(),
            kind: "DependsOn".into(),
        });
        let err = svc.remove_relationship(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn list_relationships_non_member_returns_not_found() {
        let issue = fake_issue(10, 1, 1);
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_service_non_member(issue_repo, MockIssueRelationshipRepository::new());
        let req = make_request_with_api_key(ListRelationshipsRequest { issue_slug: "TP-1".into() });
        let err = svc.list_relationships(req).await.unwrap_err();
        assert_eq!(err.code(), Code::NotFound);
    }
}
