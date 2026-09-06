use std::sync::Arc;

use super::model::{Issue, IssueFilter, IssueId, IssueStatus, IssueToken, NewIssue, NewIssueRecord, derive_issue_slug};
use crate::{
    Error, RepositoryError,
    artifact::model::{ArtifactKind, IssueArtifact, NewArtifact},
    repository::RepositoryService,
    user::{UserId, UserToken},
    with_read_only_transaction, with_transaction,
};

#[async_trait::async_trait]
pub trait IssueService: Send + Sync {
    async fn create_issue(&self, new_issue: NewIssue) -> Result<Issue, Error>;
    async fn update_issue(&self, issue: Issue) -> Result<Issue, Error>;
    async fn find_by_id(&self, id: IssueId) -> Result<Option<Issue>, Error>;
    async fn find_by_token(&self, token: IssueToken) -> Result<Option<Issue>, Error>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Issue>, Error>;
    async fn list_issues(&self, project_id: crate::project::ProjectId, filter: IssueFilter) -> Result<Vec<Issue>, Error>;
    async fn transition_issue(&self, token: IssueToken, new_status: IssueStatus, reason: Option<String>, triggered_by: Option<UserId>) -> Result<Issue, Error>;
}

pub(crate) struct IssueServiceImpl {
    repository_service: Arc<RepositoryService>,
}

impl IssueServiceImpl {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        Self { repository_service }
    }
}

#[async_trait::async_trait]
impl IssueService for IssueServiceImpl {
    async fn create_issue(&self, new_issue: NewIssue) -> Result<Issue, Error> {
        with_transaction!(self, project_repository, issue_repository, |tx| {
            // Verify project exists and atomically get next issue number.
            project_repository
                .find_by_id(tx, new_issue.project_id)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            let number = project_repository.increment_issue_counter(tx, new_issue.project_id).await?;
            let slug = derive_issue_slug(&new_issue.project_prefix, number);
            issue_repository
                .create(
                    tx,
                    NewIssueRecord {
                        project_id: new_issue.project_id,
                        number,
                        title: new_issue.title,
                        description: new_issue.description,
                        status: IssueStatus::TriageNeeded,
                        priority: new_issue.priority,
                        size: new_issue.size,
                        slug,
                        submitted_by: new_issue.submitted_by,
                    },
                )
                .await
        })
    }

    async fn update_issue(&self, issue: Issue) -> Result<Issue, Error> {
        with_transaction!(self, issue_repository, |tx| issue_repository.update(tx, issue).await)
    }

    async fn transition_issue(&self, token: IssueToken, new_status: IssueStatus, reason: Option<String>, triggered_by: Option<UserId>) -> Result<Issue, Error> {
        with_transaction!(self, issue_repository, artifact_repository, |tx| {
            let mut issue = issue_repository
                .find_by_id(tx, token.id())
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            if !issue.status.can_transition_to(&new_status) {
                return Err(Error::Validation(format!("illegal transition: {} → {}", issue.status, new_status)));
            }
            let artifacts = artifact_repository.list(tx, issue.id, None).await?;
            let auto_reason = check_transition_gate(&issue.status, &new_status, &artifacts)?;
            let final_reason = reason.or(auto_reason).unwrap_or_default();
            let from_str = issue.status.to_string();
            let to_str = new_status.to_string();

            // Update assigned based on transition direction
            if new_status.is_in_progress() {
                issue.assigned = triggered_by;
            }
            // Exiting InProgress: leave assigned unchanged (last assignee
            // retained)

            issue.status = new_status;
            let updated = issue_repository.update(tx, issue).await?;
            artifact_repository
                .create(
                    tx,
                    NewArtifact {
                        issue_id: updated.id,
                        kind: ArtifactKind::StatusTransition,
                        slug: None,
                        body: serde_json::json!({
                            "from": from_str,
                            "to": to_str,
                            "reason": final_reason,
                            "triggered_by_id": triggered_by,
                        }),
                        created_by: triggered_by.map(|id| UserToken::new(id).to_string()).unwrap_or_else(|| "system".into()),
                    },
                )
                .await?;
            Ok(updated)
        })
    }

    async fn find_by_id(&self, id: IssueId) -> Result<Option<Issue>, Error> {
        with_read_only_transaction!(self, issue_repository, |tx| issue_repository.find_by_id(tx, id).await)
    }

    async fn find_by_token(&self, token: IssueToken) -> Result<Option<Issue>, Error> {
        with_read_only_transaction!(self, issue_repository, |tx| issue_repository.find_by_id(tx, token.id()).await)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Issue>, Error> {
        let slug = slug.to_owned();
        with_read_only_transaction!(self, issue_repository, |tx| issue_repository.find_by_slug(tx, &slug).await)
    }

    async fn list_issues(&self, project_id: crate::project::ProjectId, filter: IssueFilter) -> Result<Vec<Issue>, Error> {
        with_read_only_transaction!(self, issue_repository, |tx| issue_repository.list(tx, project_id, filter).await)
    }
}

fn check_transition_gate(from: &IssueStatus, to: &IssueStatus, artifacts: &[IssueArtifact]) -> Result<Option<String>, Error> {
    use ArtifactKind::*;

    // ── Prerequisites for entering Needed states ────────────────────────────
    // These fire for any (from, to) where `to` is a gated Needed state.
    // The human is responsible for ensuring prerequisites exist, but the
    // service enforces it as a safety net.
    match to {
        IssueStatus::ResearchNeeded => {
            if !artifacts.iter().any(|a| a.kind == ResearchTopic) {
                return Err(Error::GateFailure {
                    condition: "no_research_topics".into(),
                    failing_tokens: vec![],
                });
            }
            return Ok(Some("Research topics present".into()));
        }
        IssueStatus::SpecNeeded => {
            if !artifacts.iter().any(|a| a.kind == TriageResult) {
                return Err(Error::GateFailure {
                    condition: "missing_triage_result".into(),
                    failing_tokens: vec![],
                });
            }
            return Ok(Some("Ready for spec".into()));
        }
        IssueStatus::PlanNeeded => {
            let has_triage_result = artifacts.iter().any(|a| a.kind == TriageResult);
            let has_spec = artifacts.iter().any(|a| a.kind == Spec);
            if !has_triage_result && !has_spec {
                return Err(Error::GateFailure {
                    condition: "missing_planning_input".into(),
                    failing_tokens: vec![],
                });
            }
            return Ok(Some("Planning inputs available".into()));
        }
        IssueStatus::DevNeeded => {
            if !artifacts.iter().any(|a| a.kind == Plan) {
                return Err(Error::GateFailure {
                    condition: "missing_plan".into(),
                    failing_tokens: vec![],
                });
            }
            return Ok(Some("Ready for development".into()));
        }
        // TriageNeeded → TriageInProgress has no entry prerequisite — the agent
        // freely claims triage work and produces TriageResult during the work.
        _ => {}
    }

    // ── Completion gates for InProgress → Review ────────────────────────────
    // DevInProgress → DevReview intentionally has no artifact gate here;
    // development completion is a human-reviewed judgment call.
    match (from, to) {
        // Triage agent must attach a TriageResult before moving to review.
        (IssueStatus::TriageInProgress, IssueStatus::TriageReview) => {
            if !artifacts.iter().any(|a| a.kind == TriageResult) {
                return Err(Error::GateFailure {
                    condition: "missing_triage_result".into(),
                    failing_tokens: vec![],
                });
            }
            Ok(Some("Triage result recorded".into()))
        }
        // Agent should only claim research if there are uncovered topics.
        (IssueStatus::ResearchNeeded, IssueStatus::ResearchInProgress) => {
            let topics: Vec<_> = artifacts.iter().filter(|a| a.kind == ResearchTopic).collect();
            if topics.is_empty() {
                return Err(Error::GateFailure {
                    condition: "no_research_topics".into(),
                    failing_tokens: vec![],
                });
            }
            let covered = covered_topic_tokens(artifacts);
            let uncovered: Vec<String> = topics
                .iter()
                .filter(|t| !covered.contains(&t.token.to_string()))
                .map(|t| t.token.to_string())
                .collect();
            if uncovered.is_empty() {
                return Err(Error::GateFailure {
                    condition: "all_topics_covered".into(),
                    failing_tokens: vec![],
                });
            }
            Ok(Some(format!("{} uncovered research topic(s)", uncovered.len())))
        }
        // All research topics must be covered before moving to review.
        (IssueStatus::ResearchInProgress, IssueStatus::ResearchReview) => {
            let topics: Vec<_> = artifacts.iter().filter(|a| a.kind == ResearchTopic).collect();
            if topics.is_empty() {
                return Err(Error::GateFailure {
                    condition: "no_research_topics".into(),
                    failing_tokens: vec![],
                });
            }
            let covered = covered_topic_tokens(artifacts);
            let uncovered: Vec<String> = topics
                .iter()
                .filter(|t| !covered.contains(&t.token.to_string()))
                .map(|t| t.token.to_string())
                .collect();
            if !uncovered.is_empty() {
                return Err(Error::GateFailure {
                    condition: "uncovered_research_topics".into(),
                    failing_tokens: uncovered,
                });
            }
            Ok(Some("All research topics covered".into()))
        }
        // Spec collaboration must produce a Spec artifact before review.
        (IssueStatus::SpecInProgress, IssueStatus::SpecReview) => {
            if !artifacts.iter().any(|a| a.kind == Spec) {
                return Err(Error::GateFailure {
                    condition: "missing_spec".into(),
                    failing_tokens: vec![],
                });
            }
            Ok(Some("Spec complete".into()))
        }
        // Plan agent must produce a Plan artifact before review.
        (IssueStatus::PlanInProgress, IssueStatus::PlanReview) => {
            if !artifacts.iter().any(|a| a.kind == Plan) {
                return Err(Error::GateFailure {
                    condition: "missing_plan".into(),
                    failing_tokens: vec![],
                });
            }
            Ok(Some("Plan complete".into()))
        }
        _ => Ok(None),
    }
}

fn covered_topic_tokens(artifacts: &[IssueArtifact]) -> std::collections::HashSet<String> {
    artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::Research)
        .filter_map(|a| a.body.get("topic_token").and_then(|v| v.as_str()).map(|s| s.to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;

    use super::{IssueService, IssueServiceImpl};
    use crate::{
        Error, RepositoryError,
        artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        },
        issue::{Issue, IssueFilter, IssueId, IssuePriority, IssueStatus, IssueToken, NewIssue, repository::MockIssueRepository},
        project::{ProjectId, ProjectToken, model::Project, repository::MockProjectRepository},
        repository::testing::default_repository_service_builder,
        user::UserId,
    };

    fn fake_project(id: ProjectId) -> Project {
        let now = Utc::now();
        Project {
            id,
            token: ProjectToken::new(id),
            name: "Test".into(),
            slug: "test".into(),
            prefix: "TP".into(),
            issue_counter: 0,
            description: None,
            version: 0,
            created_at: now,
            updated_at: now,
        }
    }

    fn fake_issue(id: IssueId, project_id: ProjectId, number: u32) -> Issue {
        let now = Utc::now();
        Issue {
            id,
            token: IssueToken::new(id),
            number,
            project_id,
            title: format!("Issue {number}"),
            description: "".into(),
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("TP-{number}-issue-{number}"),
            version: 0,
            submitter: 1 as UserId,
            assigned: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_svc(project_repo: MockProjectRepository, issue_repo: MockIssueRepository) -> IssueServiceImpl {
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        IssueServiceImpl::new(repo_svc)
    }

    fn make_svc_with_artifacts(
        project_repo: MockProjectRepository,
        issue_repo: MockIssueRepository,
        artifact_repo: MockArtifactRepository,
    ) -> IssueServiceImpl {
        let rs = Arc::new(
            default_repository_service_builder()
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .artifact_repository(Arc::new(artifact_repo))
                .build()
                .unwrap(),
        );
        IssueServiceImpl::new(rs)
    }

    #[tokio::test]
    async fn create_issue_success() {
        let project = fake_project(1);
        let created = fake_issue(100, 1, 1);
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_id().returning(move |_, _| {
            let p = project.clone();
            Box::pin(async move { Ok(Some(p)) })
        });
        project_repo.expect_increment_issue_counter().returning(|_, _| Box::pin(async { Ok(1u32) }));
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_create().returning(move |_, _| {
            let i = created.clone();
            Box::pin(async move { Ok(i) })
        });
        let svc = make_svc(project_repo, issue_repo);
        let result = svc
            .create_issue(NewIssue::new(1, "TP", "Fix login", "", IssuePriority::High, None, 1).unwrap())
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().number, 1);
    }

    #[tokio::test]
    async fn create_issue_project_not_found() {
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_svc(project_repo, MockIssueRepository::new());
        let result = svc
            .create_issue(NewIssue::new(999, "TP", "Fix", "", IssuePriority::Medium, None, 1).unwrap())
            .await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    #[tokio::test]
    async fn list_issues_delegates_to_repository() {
        let issues = vec![fake_issue(1, 1, 1), fake_issue(2, 1, 2)];
        let expected = issues.clone();
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_list().returning(move |_, _, _| {
            let i = expected.clone();
            Box::pin(async move { Ok(i) })
        });
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc.list_issues(1, IssueFilter::default()).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn update_issue_delegates_to_repository() {
        let issue = fake_issue(10, 1, 3);
        let updated = fake_issue(10, 1, 3);
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_update().returning(move |_, _| {
            let i = updated.clone();
            Box::pin(async move { Ok(i) })
        });
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc.update_issue(issue).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 10);
    }

    #[tokio::test]
    async fn find_by_id_delegates_to_repository() {
        let issue = fake_issue(42, 1, 5);
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_id().withf(|_, id| *id == 42).returning(move |_, _| {
            let i = issue.clone();
            Box::pin(async move { Ok(Some(i)) })
        });
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc.find_by_id(42).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 42);
    }

    #[tokio::test]
    async fn find_by_token_delegates_using_id() {
        let issue = fake_issue(42, 1, 5);
        let token = IssueToken::new(42);
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_id().withf(|_, id| *id == 42).returning(move |_, _| {
            let i = issue.clone();
            Box::pin(async move { Ok(Some(i)) })
        });
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc.find_by_token(token).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 42);
    }

    #[tokio::test]
    async fn transition_issue_success() {
        let issue = fake_issue(42, 1, 3); // status: TriageNeeded
        let transitioned = {
            let mut i = issue.clone();
            i.status = IssueStatus::TriageInProgress;
            i
        };
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().withf(|_, id| *id == 42).returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let t = transitioned.clone();
            issue_repo
                .expect_update()
                .withf(|_, i| i.status == IssueStatus::TriageInProgress)
                .returning(move |_, _| {
                    let t = t.clone();
                    Box::pin(async move { Ok(t) })
                });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));
        artifact_repo.expect_create().returning(|_, _| {
            Box::pin(async move {
                Ok(IssueArtifact {
                    id: 999,
                    token: ArtifactToken::new(999),
                    issue_id: 42,
                    kind: ArtifactKind::StatusTransition,
                    slug: None,
                    body: serde_json::json!({}),
                    created_by: "system".into(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            })
        });
        let svc = make_svc_with_artifacts(MockProjectRepository::new(), issue_repo, artifact_repo);
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::TriageInProgress, None, Some(1u64)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, IssueStatus::TriageInProgress);
    }

    #[tokio::test]
    async fn transition_issue_illegal_transition_returns_validation_error() {
        let issue = fake_issue(42, 1, 3); // status: TriageNeeded
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().withf(|_, id| *id == 42).returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        // TriageNeeded → DevReview jumps categories — illegal
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::DevReview, None, Some(1u64)).await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn transition_issue_not_found() {
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc
            .transition_issue(IssueToken::new(999), IssueStatus::TriageInProgress, None, Some(1u64))
            .await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    #[tokio::test]
    async fn transition_issue_self_transition_is_validation_error() {
        let issue = fake_issue(42, 1, 3); // status: TriageNeeded
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        // TriageNeeded → TriageNeeded is a self-transition — illegal
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::TriageNeeded, None, Some(1u64)).await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn transition_triage_in_progress_to_review_blocked_without_triage_result() {
        let mut issue = fake_issue(42, 1, 3);
        issue.status = IssueStatus::TriageInProgress;
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));
        let svc = make_svc_with_artifacts(MockProjectRepository::new(), issue_repo, artifact_repo);
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::TriageReview, None, Some(1u64)).await;
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "missing_triage_result"));
    }

    #[tokio::test]
    async fn transition_to_spec_needed_blocked_without_triage_result() {
        let mut issue = fake_issue(42, 1, 3);
        issue.status = IssueStatus::TriageReview; // valid source for → SpecNeeded
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));
        let svc = make_svc_with_artifacts(MockProjectRepository::new(), issue_repo, artifact_repo);
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::SpecNeeded, None, Some(1u64)).await;
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "missing_triage_result"));
    }

    #[tokio::test]
    async fn transition_research_in_progress_to_review_blocked_with_uncovered_topics() {
        let mut issue = fake_issue(42, 1, 3);
        issue.status = IssueStatus::ResearchInProgress;
        let topic = IssueArtifact {
            id: 1,
            token: ArtifactToken::new(1),
            issue_id: 42,
            kind: ArtifactKind::ResearchTopic,
            slug: None,
            body: serde_json::json!({"description": "Investigate auth", "tags": []}),
            created_by: "U_test".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let t = topic.clone();
            artifact_repo.expect_list().returning(move |_, _, _| {
                let t = t.clone();
                Box::pin(async move { Ok(vec![t]) })
            });
        }

        let svc = make_svc_with_artifacts(MockProjectRepository::new(), issue_repo, artifact_repo);
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::ResearchReview, None, Some(1u64)).await;
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "uncovered_research_topics"));
    }

    #[tokio::test]
    async fn transition_to_inprogress_sets_assigned() {
        let issue = fake_issue(42, 1, 3); // TriageNeeded, assigned: None
        let mut after = issue.clone();
        after.status = IssueStatus::TriageInProgress;
        after.assigned = Some(7); // triggered_by = 7

        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let a = after.clone();
            issue_repo
                .expect_update()
                .withf(|_, i| i.status == IssueStatus::TriageInProgress && i.assigned == Some(7))
                .returning(move |_, _| {
                    let a = a.clone();
                    Box::pin(async move { Ok(a) })
                });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));
        artifact_repo.expect_create().returning(|_, _| {
            Box::pin(async {
                Ok(IssueArtifact {
                    id: 1,
                    token: ArtifactToken::new(1),
                    issue_id: 42,
                    kind: ArtifactKind::StatusTransition,
                    slug: None,
                    body: serde_json::json!({}),
                    created_by: "system".into(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            })
        });
        let svc = make_svc_with_artifacts(MockProjectRepository::new(), issue_repo, artifact_repo);
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::TriageInProgress, None, Some(7)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().assigned, Some(7));
    }

    #[tokio::test]
    async fn transition_out_of_inprogress_retains_assigned() {
        let mut issue = fake_issue(42, 1, 3);
        issue.status = IssueStatus::TriageInProgress;
        issue.assigned = Some(7);

        let mut after = issue.clone();
        after.status = IssueStatus::TriageReview;
        // assigned stays Some(7) — not cleared

        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let a = after.clone();
            issue_repo
                .expect_update()
                // assigned must still be Some(7) — not cleared
                .withf(|_, i| i.status == IssueStatus::TriageReview && i.assigned == Some(7))
                .returning(move |_, _| {
                    let a = a.clone();
                    Box::pin(async move { Ok(a) })
                });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        // TriageInProgress → TriageReview requires a TriageResult
        artifact_repo.expect_list().returning(|_, _, _| {
            Box::pin(async {
                Ok(vec![IssueArtifact {
                    id: 10,
                    token: ArtifactToken::new(10),
                    issue_id: 42,
                    kind: ArtifactKind::TriageResult,
                    slug: Some("triage".into()),
                    body: serde_json::json!({"path": "some/path.md"}),
                    created_by: "system".into(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }])
            })
        });
        artifact_repo.expect_create().returning(|_, _| {
            Box::pin(async {
                Ok(IssueArtifact {
                    id: 2,
                    token: ArtifactToken::new(2),
                    issue_id: 42,
                    kind: ArtifactKind::StatusTransition,
                    slug: None,
                    body: serde_json::json!({}),
                    created_by: "system".into(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            })
        });
        let svc = make_svc_with_artifacts(MockProjectRepository::new(), issue_repo, artifact_repo);
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::TriageReview, None, Some(7)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().assigned, Some(7));
    }

    #[tokio::test]
    async fn status_transition_artifact_body_includes_triggered_by_id() {
        use crate::user::UserToken;

        let issue = fake_issue(42, 1, 3); // TriageNeeded
        let mut after = issue.clone();
        after.status = IssueStatus::TriageInProgress;
        after.assigned = Some(5);

        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let a = after.clone();
            issue_repo.expect_update().returning(move |_, _| {
                let a = a.clone();
                Box::pin(async move { Ok(a) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));
        // Capture the artifact body to assert on it
        let expected_token = UserToken::new(5u64).to_string();
        artifact_repo
            .expect_create()
            .withf(move |_, new_artifact| {
                let body = &new_artifact.body;
                body.get("triggered_by_id").and_then(|v| v.as_u64()) == Some(5) && new_artifact.created_by == expected_token
            })
            .returning(|_, _| {
                Box::pin(async {
                    Ok(IssueArtifact {
                        id: 3,
                        token: ArtifactToken::new(3),
                        issue_id: 42,
                        kind: ArtifactKind::StatusTransition,
                        slug: None,
                        body: serde_json::json!({}),
                        created_by: "system".into(),
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    })
                })
            });
        let svc = make_svc_with_artifacts(MockProjectRepository::new(), issue_repo, artifact_repo);
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::TriageInProgress, None, Some(5u64)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn status_transition_system_triggered_uses_system_created_by() {
        let issue = fake_issue(42, 1, 3); // TriageNeeded
        let mut after = issue.clone();
        after.status = IssueStatus::TriageInProgress;
        after.assigned = None;

        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let a = after.clone();
            issue_repo.expect_update().returning(move |_, _| {
                let a = a.clone();
                Box::pin(async move { Ok(a) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));
        artifact_repo
            .expect_create()
            .withf(|_, new_artifact| new_artifact.created_by == "system")
            .returning(|_, _| {
                Box::pin(async {
                    Ok(IssueArtifact {
                        id: 4,
                        token: ArtifactToken::new(4),
                        issue_id: 42,
                        kind: ArtifactKind::StatusTransition,
                        slug: None,
                        body: serde_json::json!({}),
                        created_by: "system".into(),
                        created_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    })
                })
            });
        let svc = make_svc_with_artifacts(MockProjectRepository::new(), issue_repo, artifact_repo);
        let result = svc.transition_issue(IssueToken::new(42), IssueStatus::TriageInProgress, None, None).await;
        assert!(result.is_ok());
    }

    fn make_artifact(id: u64, issue_id: u64, kind: ArtifactKind) -> IssueArtifact {
        IssueArtifact {
            id,
            token: ArtifactToken::new(id),
            issue_id,
            kind,
            slug: None,
            body: serde_json::json!({}),
            created_by: "system".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // ResearchNeeded entry gate: blocked without ResearchTopic
    #[test]
    fn gate_research_needed_blocked_without_research_topics() {
        let result = super::check_transition_gate(&IssueStatus::TriageReview, &IssueStatus::ResearchNeeded, &[]);
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "no_research_topics"));
    }

    // ResearchNeeded entry gate: passes with ResearchTopic present
    #[test]
    fn gate_research_needed_passes_with_research_topic() {
        let topic = make_artifact(1, 1, ArtifactKind::ResearchTopic);
        let result = super::check_transition_gate(&IssueStatus::TriageReview, &IssueStatus::ResearchNeeded, &[topic]);
        assert!(result.is_ok());
    }

    // PlanNeeded entry gate: blocked without triage result or spec
    #[test]
    fn gate_plan_needed_blocked_without_planning_inputs() {
        let result = super::check_transition_gate(&IssueStatus::SpecReview, &IssueStatus::PlanNeeded, &[]);
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "missing_planning_input"));
    }

    // PlanNeeded entry gate: passes with TriageResult
    #[test]
    fn gate_plan_needed_passes_with_triage_result() {
        let mut triage = make_artifact(1, 1, ArtifactKind::TriageResult);
        triage.slug = Some("triage".into());
        triage.body = serde_json::json!({"path": "triage.md"});
        let result = super::check_transition_gate(&IssueStatus::SpecReview, &IssueStatus::PlanNeeded, &[triage]);
        assert!(result.is_ok());
    }

    // PlanNeeded entry gate: passes with Spec (even without TriageResult)
    #[test]
    fn gate_plan_needed_passes_with_spec() {
        let mut spec = make_artifact(2, 1, ArtifactKind::Spec);
        spec.slug = Some("spec".into());
        spec.body = serde_json::json!({"path": "spec.md"});
        let result = super::check_transition_gate(&IssueStatus::SpecReview, &IssueStatus::PlanNeeded, &[spec]);
        assert!(result.is_ok());
    }

    // DevNeeded entry gate: blocked without Plan
    #[test]
    fn gate_dev_needed_blocked_without_plan() {
        let result = super::check_transition_gate(&IssueStatus::PlanReview, &IssueStatus::DevNeeded, &[]);
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "missing_plan"));
    }

    // DevNeeded entry gate: passes with Plan
    #[test]
    fn gate_dev_needed_passes_with_plan() {
        let mut plan = make_artifact(3, 1, ArtifactKind::Plan);
        plan.slug = Some("plan".into());
        plan.body = serde_json::json!({"path": "plan.md"});
        let result = super::check_transition_gate(&IssueStatus::PlanReview, &IssueStatus::DevNeeded, &[plan]);
        assert!(result.is_ok());
    }

    // ResearchNeeded→ResearchInProgress: all topics covered blocks transition
    #[test]
    fn gate_research_needed_to_inprogress_blocked_when_all_topics_covered() {
        let topic = make_artifact(10, 1, ArtifactKind::ResearchTopic);
        let mut research = make_artifact(11, 1, ArtifactKind::Research);
        research.body = serde_json::json!({"topic_token": topic.token.to_string(), "status": "completed", "path": "r.md"});
        let result = super::check_transition_gate(&IssueStatus::ResearchNeeded, &IssueStatus::ResearchInProgress, &[topic, research]);
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "all_topics_covered"));
    }

    // SpecInProgress→SpecReview: blocked without Spec
    #[test]
    fn gate_spec_inprogress_to_review_blocked_without_spec() {
        let result = super::check_transition_gate(&IssueStatus::SpecInProgress, &IssueStatus::SpecReview, &[]);
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "missing_spec"));
    }

    // SpecInProgress→SpecReview: passes with Spec artifact
    #[test]
    fn gate_spec_inprogress_to_review_passes_with_spec() {
        let mut spec = make_artifact(5, 1, ArtifactKind::Spec);
        spec.slug = Some("spec".into());
        spec.body = serde_json::json!({"path": "spec.md"});
        let result = super::check_transition_gate(&IssueStatus::SpecInProgress, &IssueStatus::SpecReview, &[spec]);
        assert!(result.is_ok());
    }

    // PlanInProgress→PlanReview: blocked without Plan
    #[test]
    fn gate_plan_inprogress_to_review_blocked_without_plan() {
        let result = super::check_transition_gate(&IssueStatus::PlanInProgress, &IssueStatus::PlanReview, &[]);
        assert!(matches!(result, Err(Error::GateFailure { ref condition, .. }) if condition == "missing_plan"));
    }

    // PlanInProgress→PlanReview: passes with Plan artifact
    #[test]
    fn gate_plan_inprogress_to_review_passes_with_plan() {
        let mut plan = make_artifact(6, 1, ArtifactKind::Plan);
        plan.slug = Some("plan".into());
        plan.body = serde_json::json!({"path": "plan.md"});
        let result = super::check_transition_gate(&IssueStatus::PlanInProgress, &IssueStatus::PlanReview, &[plan]);
        assert!(result.is_ok());
    }
}
