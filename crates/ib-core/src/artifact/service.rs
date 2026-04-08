use std::{collections::HashSet, sync::Arc};

use serde_json::Value;

use crate::{
    Error, RepositoryError,
    artifact::model::{ArtifactKind, ArtifactToken, IssueArtifact, NewArtifact},
    issue::IssueId,
    repository::RepositoryService,
    with_read_only_transaction, with_transaction,
};

#[async_trait::async_trait]
pub trait ArtifactService: Send + Sync {
    async fn add_artifact(&self, new_artifact: NewArtifact) -> Result<IssueArtifact, Error>;
    async fn update_artifact(&self, token: ArtifactToken, body: Value) -> Result<IssueArtifact, Error>;
    async fn remove_artifact(&self, token: ArtifactToken) -> Result<(), Error>;
    async fn list_artifacts(&self, issue_id: IssueId, kinds: Option<Vec<ArtifactKind>>, uncovered_only: bool) -> Result<Vec<IssueArtifact>, Error>;
}

pub(crate) struct ArtifactServiceImpl {
    repository_service: Arc<RepositoryService>,
}

impl ArtifactServiceImpl {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        Self { repository_service }
    }
}

#[async_trait::async_trait]
impl ArtifactService for ArtifactServiceImpl {
    async fn add_artifact(&self, new_artifact: NewArtifact) -> Result<IssueArtifact, Error> {
        if new_artifact.kind == ArtifactKind::StatusTransition {
            return Err(Error::Validation("StatusTransition artifacts are system-generated".into()));
        }
        validate_body(&new_artifact.kind, &new_artifact.body)?;
        with_transaction!(self, artifact_repository, |tx| artifact_repository.create(tx, new_artifact).await)
    }

    async fn update_artifact(&self, token: ArtifactToken, body: Value) -> Result<IssueArtifact, Error> {
        with_transaction!(self, artifact_repository, |tx| {
            let mut artifact = artifact_repository
                .find_by_token(tx, token)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            if artifact.kind == ArtifactKind::StatusTransition {
                return Err(Error::Validation("StatusTransition artifacts are immutable".into()));
            }
            if is_file_backed(&artifact.kind) {
                let old_path = artifact.body.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let new_path = body.get("path").and_then(|v| v.as_str()).unwrap_or("");
                if !old_path.is_empty() && old_path != new_path {
                    return Err(Error::Validation("file path is immutable after creation".into()));
                }
            }
            validate_body(&artifact.kind, &body)?;
            artifact.body = body;
            artifact_repository.update(tx, artifact).await
        })
    }

    async fn remove_artifact(&self, token: ArtifactToken) -> Result<(), Error> {
        with_transaction!(self, artifact_repository, |tx| {
            let artifact = artifact_repository
                .find_by_token(tx, token)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            artifact_repository.delete(tx, artifact.id).await
        })
    }

    async fn list_artifacts(&self, issue_id: IssueId, kinds: Option<Vec<ArtifactKind>>, uncovered_only: bool) -> Result<Vec<IssueArtifact>, Error> {
        with_read_only_transaction!(self, artifact_repository, |tx| {
            let artifacts = artifact_repository.list(tx, issue_id, kinds.clone()).await?;
            if !uncovered_only {
                return Ok(artifacts);
            }
            // Fetch Research artifacts to compute coverage (they may not be in `kinds`)
            let requesting_research = kinds.as_ref().is_none_or(|k| k.contains(&ArtifactKind::Research));
            let research: Vec<IssueArtifact> = if requesting_research {
                artifacts.iter().filter(|a| a.kind == ArtifactKind::Research).cloned().collect()
            } else {
                artifact_repository.list(tx, issue_id, Some(vec![ArtifactKind::Research])).await?
            };
            let covered: HashSet<String> = research
                .iter()
                .filter_map(|a| a.body.get("topic_token").and_then(|v| v.as_str()).map(|s| s.to_owned()))
                .collect();
            Ok(artifacts
                .into_iter()
                .filter(|a| {
                    if a.kind == ArtifactKind::ResearchTopic {
                        !covered.contains(&a.token.to_string())
                    } else {
                        true
                    }
                })
                .collect())
        })
    }
}

fn is_file_backed(kind: &ArtifactKind) -> bool {
    matches!(
        kind,
        ArtifactKind::TriageResult | ArtifactKind::Spec | ArtifactKind::Research | ArtifactKind::Plan
    )
}

pub(crate) fn validate_body(kind: &ArtifactKind, body: &Value) -> Result<(), Error> {
    match kind {
        ArtifactKind::TriageResult | ArtifactKind::Spec | ArtifactKind::Plan => {
            if body.get("path").and_then(|v| v.as_str()).is_none() {
                return Err(Error::Validation(format!("{kind} body requires a 'path' field")));
            }
        }
        ArtifactKind::Research => {
            if body.get("topic_token").and_then(|v| v.as_str()).is_none() {
                return Err(Error::Validation("Research body requires 'topic_token'".into()));
            }
            let status = body.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if status != "completed" && status != "cancelled" {
                return Err(Error::Validation("Research 'status' must be 'completed' or 'cancelled'".into()));
            }
            if status == "completed" && body.get("path").and_then(|v| v.as_str()).is_none() {
                return Err(Error::Validation("Research with status 'completed' requires 'path'".into()));
            }
        }
        ArtifactKind::ResearchTopic => {
            let has_desc = body.get("description").and_then(|v| v.as_str()).is_some();
            let has_path = body.get("path").and_then(|v| v.as_str()).is_some();
            if !has_desc && !has_path {
                return Err(Error::Validation("ResearchTopic body requires 'description' or 'path'".into()));
            }
            if has_desc && has_path {
                return Err(Error::Validation("ResearchTopic body must have 'description' or 'path', not both".into()));
            }
        }
        ArtifactKind::Comment => {
            if body.get("text").and_then(|v| v.as_str()).is_none() {
                return Err(Error::Validation("Comment body requires 'text'".into()));
            }
        }
        ArtifactKind::StatusTransition => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use serde_json::json;

    use super::*;
    use crate::{
        artifact::{
            model::{ArtifactToken, IssueArtifact, NewArtifact},
            repository::MockArtifactRepository,
        },
        repository::testing::default_repository_service_builder,
    };

    fn fake_artifact(id: u64, issue_id: u64, kind: ArtifactKind, body: serde_json::Value) -> IssueArtifact {
        IssueArtifact {
            id,
            token: ArtifactToken::new(id),
            issue_id,
            kind,
            body,
            created_by: "U_test".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_svc(artifact_repo: MockArtifactRepository) -> ArtifactServiceImpl {
        let rs = default_repository_service_builder()
            .artifact_repository(Arc::new(artifact_repo))
            .build()
            .unwrap();
        ArtifactServiceImpl::new(Arc::new(rs))
    }

    #[tokio::test]
    async fn add_artifact_rejects_status_transition() {
        let svc = make_svc(MockArtifactRepository::new());
        let result = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::StatusTransition,
                body: json!({"from": "Triage", "to": "SpecNeeded"}),
                created_by: "U_1".into(),
            })
            .await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn add_artifact_rejects_research_missing_topic_token() {
        let svc = make_svc(MockArtifactRepository::new());
        let result = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::Research,
                body: json!({"status": "completed", "path": "insights/research/foo.md"}),
                created_by: "U_1".into(),
            })
            .await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn list_artifacts_uncovered_only_excludes_covered_topics() {
        let topic1 = fake_artifact(1, 42, ArtifactKind::ResearchTopic, json!({"description": "topic A", "tags": []}));
        let topic2 = fake_artifact(2, 42, ArtifactKind::ResearchTopic, json!({"description": "topic B", "tags": []}));
        let research1 = fake_artifact(
            3,
            42,
            ArtifactKind::Research,
            json!({"topic_token": topic1.token.to_string(), "status": "completed", "path": "insights/research/a.md"}),
        );
        let all = vec![topic1.clone(), topic2.clone(), research1];

        let mut repo = MockArtifactRepository::new();
        repo.expect_list().returning(move |_, _, _| {
            let a = all.clone();
            Box::pin(async move { Ok(a) })
        });

        let svc = make_svc(repo);
        let result = svc
            .list_artifacts(42, Some(vec![ArtifactKind::ResearchTopic, ArtifactKind::Research]), true)
            .await
            .unwrap();

        // Covered topic1 filtered out; uncovered topic2 and Research artifact pass
        // through
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|a| a.id == topic2.id));
        assert!(!result.iter().any(|a| a.id == topic1.id));
    }

    #[tokio::test]
    async fn list_artifacts_uncovered_only_passes_through_non_topic_artifacts() {
        let topic = fake_artifact(1, 42, ArtifactKind::ResearchTopic, json!({"description": "topic A", "tags": []}));
        let research = fake_artifact(
            2,
            42,
            ArtifactKind::Research,
            json!({"topic_token": topic.token.to_string(), "status": "completed", "path": "insights/research/a.md"}),
        );
        let comment = fake_artifact(3, 42, ArtifactKind::Comment, json!({"text": "a note"}));
        // topic is covered, research and comment should pass through
        let all = vec![topic.clone(), research, comment.clone()];

        let mut repo = MockArtifactRepository::new();
        repo.expect_list().returning(move |_, _, _| {
            let a = all.clone();
            Box::pin(async move { Ok(a) })
        });

        let svc = make_svc(repo);
        let result = svc.list_artifacts(42, None, true).await.unwrap();

        // Covered topic filtered out; Research and Comment pass through
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|a| a.id == comment.id));
    }
}
