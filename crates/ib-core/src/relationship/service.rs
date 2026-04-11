use std::{collections::VecDeque, sync::Arc};

use super::model::{IssueRelationship, IssueRelationships, NewIssueRelationship, RelationshipKind};
use crate::{Error, RepositoryError, issue::IssueId, repository::RepositoryService, with_read_only_transaction, with_transaction};

#[async_trait::async_trait]
pub trait IssueRelationshipService: Send + Sync {
    async fn add_relationship(&self, from_slug: &str, to_slug: &str, kind: RelationshipKind) -> Result<IssueRelationship, Error>;

    async fn remove_relationship(&self, from_slug: &str, to_slug: &str, kind: RelationshipKind) -> Result<bool, Error>;

    async fn list_for_issue(&self, issue_id: IssueId) -> Result<IssueRelationships, Error>;
}

pub(crate) struct IssueRelationshipServiceImpl {
    repository_service: Arc<RepositoryService>,
}

impl IssueRelationshipServiceImpl {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        Self { repository_service }
    }
}

#[async_trait::async_trait]
impl IssueRelationshipService for IssueRelationshipServiceImpl {
    async fn add_relationship(&self, from_slug: &str, to_slug: &str, kind: RelationshipKind) -> Result<IssueRelationship, Error> {
        let from_slug = from_slug.to_owned();
        let to_slug = to_slug.to_owned();
        with_transaction!(self, issue_repository, relationship_repository, |tx| {
            let from = issue_repository
                .find_by_slug(tx, &from_slug)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            let to = issue_repository
                .find_by_slug(tx, &to_slug)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;

            if from.id == to.id {
                return Err(Error::Validation("an issue cannot relate to itself".into()));
            }
            if from.project_id != to.project_id {
                return Err(Error::Validation("relationships are only allowed within the same project".into()));
            }

            if kind == RelationshipKind::DependsOn {
                // BFS from `to.id`: if we can reach `from.id`, adding this edge creates a
                // cycle.
                let mut visited = std::collections::HashSet::new();
                let mut queue = VecDeque::new();
                queue.push_back(to.id);

                while let Some(current) = queue.pop_front() {
                    if current == from.id {
                        return Err(Error::CycleDetected);
                    }
                    if !visited.insert(current) {
                        continue;
                    }
                    // Follow DependsOn edges outward from `current`
                    let rels = relationship_repository.list_for_issue(tx, current).await?;
                    for dep in rels.depends_on {
                        if !visited.contains(&dep.id) {
                            queue.push_back(dep.id);
                        }
                    }
                }
            }

            relationship_repository
                .add(
                    tx,
                    NewIssueRelationship {
                        from_issue_id: from.id,
                        to_issue_id: to.id,
                        kind,
                    },
                )
                .await
                .map_err(|e| match e {
                    Error::RepositoryError(RepositoryError::Constraint(_)) => Error::AlreadyExists,
                    other => other,
                })
        })
    }

    async fn remove_relationship(&self, from_slug: &str, to_slug: &str, kind: RelationshipKind) -> Result<bool, Error> {
        let from_slug = from_slug.to_owned();
        let to_slug = to_slug.to_owned();
        with_transaction!(self, issue_repository, relationship_repository, |tx| {
            let from = issue_repository
                .find_by_slug(tx, &from_slug)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            let to = issue_repository
                .find_by_slug(tx, &to_slug)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;

            relationship_repository.remove(tx, from.id, to.id, kind).await
        })
    }

    async fn list_for_issue(&self, issue_id: IssueId) -> Result<IssueRelationships, Error> {
        with_read_only_transaction!(self, relationship_repository, |tx| {
            relationship_repository.list_for_issue(tx, issue_id).await
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use mockall::predicate::*;

    use super::*;
    use crate::{
        Error,
        issue::{IssuePriority, IssueStatus, IssueToken, MockIssueRepository},
        relationship::{
            MockIssueRelationshipRepository,
            model::{IssueRelationship, IssueRelationships, RelatedIssueSummary, RelationshipKind},
        },
        repository::testing::default_repository_service_builder,
    };

    fn fake_issue(id: u64, project_id: u64, slug: &str) -> crate::issue::Issue {
        crate::issue::Issue {
            id,
            token: IssueToken::new(id),
            number: id as u32,
            project_id,
            title: format!("Issue {id}"),
            description: String::new(),
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: slug.to_owned(),
            version: 0,
            submitter: 1,
            assigned: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_service(issue_repo: MockIssueRepository, rel_repo: MockIssueRelationshipRepository) -> IssueRelationshipServiceImpl {
        let rs = Arc::new(
            default_repository_service_builder()
                .issue_repository(Arc::new(issue_repo))
                .relationship_repository(Arc::new(rel_repo) as Arc<dyn crate::relationship::IssueRelationshipRepository>)
                .build()
                .unwrap(),
        );
        IssueRelationshipServiceImpl::new(rs)
    }

    #[tokio::test]
    async fn self_relationship_is_rejected() {
        let mut issue_repo = MockIssueRepository::new();
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("A-1"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(1, 1, "A-1"))) }));

        let svc = make_service(issue_repo, MockIssueRelationshipRepository::new());
        let err = svc.add_relationship("A-1", "A-1", RelationshipKind::DependsOn).await.unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "expected Validation, got {err:?}");
    }

    #[tokio::test]
    async fn cross_project_is_rejected() {
        let mut issue_repo = MockIssueRepository::new();
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("A-1"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(1, 1, "A-1"))) }));
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("B-1"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(2, 2, "B-1"))) }));

        let svc = make_service(issue_repo, MockIssueRelationshipRepository::new());
        let err = svc.add_relationship("A-1", "B-1", RelationshipKind::DependsOn).await.unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "expected Validation, got {err:?}");
    }

    #[tokio::test]
    async fn direct_cycle_detected() {
        // A→B exists; trying to add B→A should fail
        let mut issue_repo = MockIssueRepository::new();
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("A-2"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(2, 1, "A-2"))) }));
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("A-1"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(1, 1, "A-1"))) }));

        let mut rel_repo = MockIssueRelationshipRepository::new();
        // BFS: from to_issue_id=1, check list_for_issue(1).depends_on → includes id=2 →
        // cycle
        rel_repo.expect_list_for_issue().returning(|_, issue_id| {
            Box::pin(async move {
                if issue_id == 1 {
                    // issue 1 depends_on issue 2 (the from_issue_id)
                    Ok(IssueRelationships {
                        depends_on: vec![RelatedIssueSummary {
                            id: 2,
                            slug: "A-2".to_owned(),
                            title: "Issue 2".to_owned(),
                        }],
                        blocks: vec![],
                        related_to: vec![],
                    })
                } else {
                    Ok(IssueRelationships::default())
                }
            })
        });

        let svc = make_service(issue_repo, rel_repo);
        let err = svc.add_relationship("A-2", "A-1", RelationshipKind::DependsOn).await.unwrap_err();
        assert!(matches!(err, Error::CycleDetected), "expected CycleDetected, got {err:?}");
    }

    #[tokio::test]
    async fn transitive_cycle_detected() {
        // Graph: A→B→C exists. Adding C→A should detect a cycle via BFS.
        // from_slug = "A-3" (id=3), to_slug = "A-1" (id=1)
        // BFS starts at to=1:
        //   - list_for_issue(1).depends_on → [id=2] → enqueue 2
        //   - list_for_issue(2).depends_on → [id=3] → enqueue 3
        //   - current=3 == from.id=3 → CycleDetected
        let mut issue_repo = MockIssueRepository::new();
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("A-3"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(3, 1, "A-3"))) }));
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("A-1"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(1, 1, "A-1"))) }));

        let mut rel_repo = MockIssueRelationshipRepository::new();
        rel_repo.expect_list_for_issue().returning(|_, issue_id| {
            Box::pin(async move {
                match issue_id {
                    // issue 1 (A-1) depends on issue 2
                    1 => Ok(IssueRelationships {
                        depends_on: vec![RelatedIssueSummary {
                            id: 2,
                            slug: "A-2".to_owned(),
                            title: "Issue 2".to_owned(),
                        }],
                        blocks: vec![],
                        related_to: vec![],
                    }),
                    // issue 2 (A-2) depends on issue 3 (the from_issue_id)
                    2 => Ok(IssueRelationships {
                        depends_on: vec![RelatedIssueSummary {
                            id: 3,
                            slug: "A-3".to_owned(),
                            title: "Issue 3".to_owned(),
                        }],
                        blocks: vec![],
                        related_to: vec![],
                    }),
                    _ => Ok(IssueRelationships::default()),
                }
            })
        });

        let svc = make_service(issue_repo, rel_repo);
        let err = svc.add_relationship("A-3", "A-1", RelationshipKind::DependsOn).await.unwrap_err();
        assert!(matches!(err, Error::CycleDetected), "expected CycleDetected, got {err:?}");
    }

    #[tokio::test]
    async fn related_to_skips_cycle_check() {
        let mut issue_repo = MockIssueRepository::new();
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("A-1"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(1, 1, "A-1"))) }));
        issue_repo
            .expect_find_by_slug()
            .with(always(), eq("A-2"))
            .returning(|_, _| Box::pin(async { Ok(Some(fake_issue(2, 1, "A-2"))) }));

        let mut rel_repo = MockIssueRelationshipRepository::new();
        // list_for_issue should NOT be called for RelatedTo
        rel_repo.expect_list_for_issue().never();
        rel_repo.expect_add().returning(|_, rec| {
            let kind = rec.kind.clone();
            let from = rec.from_issue_id;
            let to = rec.to_issue_id;
            Box::pin(async move {
                Ok(IssueRelationship {
                    id: 99,
                    from_issue_id: from,
                    to_issue_id: to,
                    kind,
                    created_at: chrono::Utc::now(),
                })
            })
        });

        let svc = make_service(issue_repo, rel_repo);
        svc.add_relationship("A-1", "A-2", RelationshipKind::RelatedTo).await.unwrap();
    }
}
