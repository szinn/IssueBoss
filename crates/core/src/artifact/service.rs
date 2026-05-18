use std::{collections::HashSet, sync::Arc};

use serde_json::Value;

use crate::{
    Error, RepositoryError,
    artifact::model::{ArtifactId, ArtifactKind, IssueArtifact, NewArtifact},
    issue::IssueId,
    repository::RepositoryService,
    with_read_only_transaction, with_transaction,
};

#[async_trait::async_trait]
pub trait ArtifactService: Send + Sync {
    async fn add_artifact(&self, new_artifact: NewArtifact) -> Result<IssueArtifact, Error>;
    async fn update_artifact(&self, issue_id: IssueId, slug: &str, body: Value) -> Result<IssueArtifact, Error>;
    async fn update_artifact_by_id(&self, issue_id: IssueId, artifact_id: ArtifactId, body: Value) -> Result<IssueArtifact, Error>;
    async fn remove_artifact(&self, issue_id: IssueId, slug: &str) -> Result<(), Error>;
    async fn remove_artifact_by_id(&self, issue_id: IssueId, artifact_id: ArtifactId) -> Result<(), Error>;
    async fn list_artifacts(&self, issue_id: IssueId, kinds: Option<Vec<ArtifactKind>>, uncovered_only: bool) -> Result<Vec<IssueArtifact>, Error>;
    async fn move_artifact(&self, old_path: &str, new_path: &str) -> Result<Vec<IssueArtifact>, Error>;
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
    async fn add_artifact(&self, mut new_artifact: NewArtifact) -> Result<IssueArtifact, Error> {
        if new_artifact.kind == ArtifactKind::StatusTransition {
            return Err(Error::Validation("StatusTransition artifacts are system-generated".into()));
        }
        new_artifact.slug = match new_artifact.kind {
            ArtifactKind::TriageResult => Some("triage".to_string()),
            ArtifactKind::Spec => Some("spec".to_string()),
            ArtifactKind::Plan => Some("plan".to_string()),
            _ => {
                let slug = new_artifact
                    .slug
                    .take()
                    .ok_or_else(|| Error::Validation(format!("{} artifact requires a slug", new_artifact.kind)))?;
                validate_slug(&slug)?;
                Some(slug)
            }
        };
        validate_body(&new_artifact.kind, &mut new_artifact.body)?;
        with_transaction!(self, artifact_repository, |tx| artifact_repository.create(tx, new_artifact).await)
    }

    async fn update_artifact(&self, issue_id: IssueId, slug: &str, mut body: Value) -> Result<IssueArtifact, Error> {
        let slug = slug.to_owned();
        with_transaction!(self, artifact_repository, |tx| {
            let mut artifact = artifact_repository
                .find_by_slug(tx, issue_id, &slug)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            validate_artifact_update(&artifact, &mut body)?;
            artifact.body = body;
            artifact_repository.update(tx, artifact).await
        })
    }

    async fn update_artifact_by_id(&self, issue_id: IssueId, artifact_id: ArtifactId, mut body: Value) -> Result<IssueArtifact, Error> {
        with_transaction!(self, artifact_repository, |tx| {
            let mut artifact = artifact_repository
                .find_by_id(tx, artifact_id)
                .await?
                .filter(|a| a.issue_id == issue_id)
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            validate_artifact_update(&artifact, &mut body)?;
            artifact.body = body;
            artifact_repository.update(tx, artifact).await
        })
    }

    async fn remove_artifact(&self, issue_id: IssueId, slug: &str) -> Result<(), Error> {
        let slug = slug.to_owned();
        with_transaction!(self, artifact_repository, |tx| {
            let artifact = artifact_repository
                .find_by_slug(tx, issue_id, &slug)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            artifact_repository.delete(tx, artifact.id).await
        })
    }

    async fn remove_artifact_by_id(&self, issue_id: IssueId, artifact_id: ArtifactId) -> Result<(), Error> {
        with_transaction!(self, artifact_repository, |tx| {
            let artifact = artifact_repository
                .find_by_id(tx, artifact_id)
                .await?
                .filter(|a| a.issue_id == issue_id)
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

    async fn move_artifact(&self, old_path: &str, new_path: &str) -> Result<Vec<IssueArtifact>, Error> {
        if old_path == new_path {
            return Ok(vec![]);
        }
        let old_path = old_path.to_owned();
        let new_path = new_path.to_owned();
        with_transaction!(self, artifact_repository, |tx| {
            let artifacts = artifact_repository.find_by_path(tx, &old_path).await?;
            let mut results = Vec::with_capacity(artifacts.len());
            for mut artifact in artifacts {
                if let Some(obj) = artifact.body.as_object_mut() {
                    obj.insert("path".to_owned(), serde_json::Value::String(new_path.clone()));
                }
                let updated = artifact_repository.update(tx, artifact).await?;
                results.push(updated);
            }
            Ok(results)
        })
    }
}

fn validate_artifact_update(artifact: &IssueArtifact, body: &mut Value) -> Result<(), Error> {
    if artifact.kind == ArtifactKind::StatusTransition {
        return Err(Error::Validation("StatusTransition artifacts are immutable".into()));
    }
    validate_body(&artifact.kind, body)?;
    if is_file_backed(&artifact.kind) {
        let old_path = artifact.body.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let new_path = body.get("path").and_then(|v| v.as_str()).unwrap_or("");
        if !old_path.is_empty() && old_path != new_path {
            return Err(Error::Validation("file path is immutable after creation".into()));
        }
    }
    Ok(())
}

fn is_file_backed(kind: &ArtifactKind) -> bool {
    matches!(
        kind,
        ArtifactKind::TriageResult | ArtifactKind::Spec | ArtifactKind::Research | ArtifactKind::Plan | ArtifactKind::Handoff
    )
}

pub(crate) fn validate_body(kind: &ArtifactKind, body: &mut Value) -> Result<(), Error> {
    if let Some(s) = body.as_str() {
        let parsed: Value = serde_json::from_str(s).map_err(|_| Error::Validation(format!("{kind} body must be a JSON object")))?;
        if !parsed.is_object() {
            return Err(Error::Validation(format!("{kind} body must be a JSON object")));
        }
        *body = parsed;
    } else if !body.is_object() && !matches!(kind, ArtifactKind::StatusTransition) {
        return Err(Error::Validation(format!("{kind} body must be a JSON object")));
    }
    match kind {
        ArtifactKind::TriageResult | ArtifactKind::Spec | ArtifactKind::Plan | ArtifactKind::Handoff => {
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

fn validate_slug(slug: &str) -> Result<(), Error> {
    if slug.is_empty() {
        return Err(Error::Validation("slug must not be empty".into()));
    }
    if !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(Error::Validation("slug must contain only lowercase letters, digits, and hyphens".into()));
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
            slug: None,
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
                slug: None,
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
                slug: Some("test-topic".into()),
                body: json!({"status": "completed", "path": "insights/research/foo.md"}),
                created_by: "U_1".into(),
            })
            .await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn add_artifact_rejects_multi_type_without_slug() {
        let svc = make_svc(MockArtifactRepository::new());
        let result = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::ResearchTopic,
                slug: None,
                body: json!({"description": "investigate auth"}),
                created_by: "U_1".into(),
            })
            .await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn add_artifact_rejects_invalid_slug() {
        let svc = make_svc(MockArtifactRepository::new());
        // Uppercase letters are not allowed in slugs
        let result = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::Comment,
                slug: Some("UPPER_CASE".into()),
                body: json!({"text": "a comment"}),
                created_by: "U_1".into(),
            })
            .await;
        assert!(matches!(result, Err(Error::Validation(_))));
        // Underscores are not allowed either
        let result = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::Comment,
                slug: Some("under_score".into()),
                body: json!({"text": "a comment"}),
                created_by: "U_1".into(),
            })
            .await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn add_artifact_auto_assigns_triage_slug() {
        let mut repo = MockArtifactRepository::new();
        repo.expect_create().returning(|_, record| {
            let slug = record.slug.clone();
            let kind = record.kind.clone();
            let issue_id = record.issue_id;
            let body = record.body.clone();
            let created_by = record.created_by.clone();
            Box::pin(async move {
                Ok(IssueArtifact {
                    id: 1,
                    token: ArtifactToken::new(1),
                    issue_id,
                    kind,
                    slug,
                    body,
                    created_by,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            })
        });
        let svc = make_svc(repo);
        let artifact = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::TriageResult,
                slug: None,
                body: json!({"path": "insights/triage/t-1.md"}),
                created_by: "U_1".into(),
            })
            .await
            .unwrap();
        assert_eq!(artifact.slug, Some("triage".to_string()));
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

    #[tokio::test]
    async fn move_artifact_noop_when_paths_equal() {
        // No repository expectations — DB must not be touched
        let svc = make_svc(MockArtifactRepository::new());
        let result = svc.move_artifact(".insights/spec.md", ".insights/spec.md").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn move_artifact_returns_empty_when_no_artifacts_match() {
        let mut repo = MockArtifactRepository::new();
        repo.expect_find_by_path().returning(|_, _| Box::pin(async { Ok(vec![]) }));
        let svc = make_svc(repo);
        let result = svc.move_artifact(".insights/old.md", ".insights/new.md").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn move_artifact_updates_all_matching_artifacts() {
        let artifact1 = fake_artifact(1, 10, ArtifactKind::Spec, json!({"path": ".insights/old.md"}));
        let artifact2 = fake_artifact(2, 11, ArtifactKind::TriageResult, json!({"path": ".insights/old.md"}));

        let a1 = artifact1.clone();
        let a2 = artifact2.clone();
        let mut repo = MockArtifactRepository::new();
        repo.expect_find_by_path().returning(move |_, _| {
            let v = vec![a1.clone(), a2.clone()];
            Box::pin(async move { Ok(v) })
        });
        repo.expect_update().times(2).returning(|_, a| Box::pin(async move { Ok(a) }));

        let svc = make_svc(repo);
        let result = svc.move_artifact(".insights/old.md", ".insights/new.md").await.unwrap();

        assert_eq!(result.len(), 2);
        for artifact in &result {
            assert_eq!(artifact.body["path"], ".insights/new.md");
        }
    }

    #[tokio::test]
    async fn add_artifact_rejects_handoff_missing_path() {
        let svc = make_svc(MockArtifactRepository::new());
        let result = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::Handoff,
                slug: Some("phase-1-complete".into()),
                body: json!({"note": "missing path"}),
                created_by: "U_1".into(),
            })
            .await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn add_artifact_rejects_handoff_without_slug() {
        let svc = make_svc(MockArtifactRepository::new());
        let result = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::Handoff,
                slug: None,
                body: json!({"path": ".insights/issues/IB-1-handoff.md"}),
                created_by: "U_1".into(),
            })
            .await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn handoff_is_file_backed() {
        assert!(is_file_backed(&ArtifactKind::Handoff));
    }

    // validate_body edge cases

    #[test]
    fn validate_body_comment_requires_text() {
        let mut body = json!({"note": "oops"});
        let result = validate_body(&ArtifactKind::Comment, &mut body);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn validate_body_comment_with_text_is_ok() {
        let mut body = json!({"text": "hello"});
        assert!(validate_body(&ArtifactKind::Comment, &mut body).is_ok());
    }

    #[test]
    fn validate_body_research_missing_topic_token() {
        let mut body = json!({"status": "completed", "path": "foo.md"});
        let result = validate_body(&ArtifactKind::Research, &mut body);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn validate_body_research_invalid_status() {
        let mut body = json!({"topic_token": "T_1", "status": "pending"});
        let result = validate_body(&ArtifactKind::Research, &mut body);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn validate_body_research_completed_requires_path() {
        let mut body = json!({"topic_token": "T_1", "status": "completed"});
        let result = validate_body(&ArtifactKind::Research, &mut body);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn validate_body_research_cancelled_no_path_required() {
        let mut body = json!({"topic_token": "T_1", "status": "cancelled"});
        assert!(validate_body(&ArtifactKind::Research, &mut body).is_ok());
    }

    #[test]
    fn validate_body_research_topic_both_desc_and_path_rejected() {
        let mut body = json!({"description": "desc", "path": "foo.md"});
        let result = validate_body(&ArtifactKind::ResearchTopic, &mut body);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn validate_body_research_topic_neither_desc_nor_path_rejected() {
        let mut body = json!({});
        let result = validate_body(&ArtifactKind::ResearchTopic, &mut body);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn validate_body_research_topic_desc_only_ok() {
        let mut body = json!({"description": "what is auth?"});
        assert!(validate_body(&ArtifactKind::ResearchTopic, &mut body).is_ok());
    }

    #[test]
    fn validate_body_status_transition_always_ok() {
        // StatusTransition has no body validation
        let mut body = json!({});
        assert!(validate_body(&ArtifactKind::StatusTransition, &mut body).is_ok());
    }

    // String-body coercion: MCP clients sometimes serialize object params as
    // JSON-encoded strings; validate_body should parse and replace in place.

    #[test]
    fn validate_body_coerces_string_object_for_triage_result() {
        let mut body = Value::String(r#"{"path":"foo.md"}"#.to_string());
        assert!(validate_body(&ArtifactKind::TriageResult, &mut body).is_ok());
        assert_eq!(body, json!({"path": "foo.md"}));
    }

    #[test]
    fn validate_body_rejects_non_json_string() {
        let mut body = Value::String("not json".to_string());
        let result = validate_body(&ArtifactKind::TriageResult, &mut body);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn validate_body_rejects_string_that_parses_to_non_object() {
        // valid JSON, but not an object
        let mut body = Value::String("\"just a string\"".to_string());
        let result = validate_body(&ArtifactKind::TriageResult, &mut body);
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[test]
    fn validate_body_coerces_string_object_for_research() {
        let mut body = Value::String(r#"{"topic_token":"T_1","status":"completed","path":"r.md"}"#.to_string());
        assert!(validate_body(&ArtifactKind::Research, &mut body).is_ok());
        assert_eq!(body, json!({"topic_token": "T_1", "status": "completed", "path": "r.md"}));
    }

    #[test]
    fn validate_body_coerces_string_object_for_comment() {
        let mut body = Value::String(r#"{"text":"hi"}"#.to_string());
        assert!(validate_body(&ArtifactKind::Comment, &mut body).is_ok());
        assert_eq!(body, json!({"text": "hi"}));
    }

    #[test]
    fn validate_body_leaves_object_unchanged() {
        let mut body = json!({"path": "x.md"});
        let original = body.clone();
        assert!(validate_body(&ArtifactKind::TriageResult, &mut body).is_ok());
        assert_eq!(body, original);
    }

    // update_artifact guards

    #[tokio::test]
    async fn update_artifact_rejects_status_transition_kind() {
        let mut repo = MockArtifactRepository::new();
        let artifact = fake_artifact(5, 10, ArtifactKind::StatusTransition, json!({"from": "A", "to": "B"}));
        let a = artifact.clone();
        repo.expect_find_by_slug().returning(move |_, _, _| {
            let a = a.clone();
            Box::pin(async move { Ok(Some(a)) })
        });
        let svc = make_svc(repo);
        let result = svc.update_artifact(10, "some-slug", json!({"from": "A", "to": "C"})).await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn update_artifact_rejects_file_path_change() {
        let mut repo = MockArtifactRepository::new();
        let artifact = fake_artifact(6, 10, ArtifactKind::Spec, json!({"path": ".insights/spec.md"}));
        let a = artifact.clone();
        repo.expect_find_by_slug().returning(move |_, _, _| {
            let a = a.clone();
            Box::pin(async move { Ok(Some(a)) })
        });
        let svc = make_svc(repo);
        let result = svc.update_artifact(10, "spec", json!({"path": ".insights/spec-v2.md"})).await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    // update_artifact_by_id issue_id mismatch and guards

    #[tokio::test]
    async fn update_artifact_by_id_issue_id_mismatch_returns_not_found() {
        let mut repo = MockArtifactRepository::new();
        // artifact belongs to issue 99, but we pass issue_id=10
        let artifact = fake_artifact(7, 99, ArtifactKind::Comment, json!({"text": "hi"}));
        let a = artifact.clone();
        repo.expect_find_by_id().returning(move |_, _| {
            let a = a.clone();
            Box::pin(async move { Ok(Some(a)) })
        });
        let svc = make_svc(repo);
        let result = svc.update_artifact_by_id(10, 7, json!({"text": "updated"})).await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    #[tokio::test]
    async fn update_artifact_by_id_rejects_status_transition_kind() {
        let mut repo = MockArtifactRepository::new();
        let artifact = fake_artifact(8, 10, ArtifactKind::StatusTransition, json!({"from": "A", "to": "B"}));
        let a = artifact.clone();
        repo.expect_find_by_id().returning(move |_, _| {
            let a = a.clone();
            Box::pin(async move { Ok(Some(a)) })
        });
        let svc = make_svc(repo);
        let result = svc.update_artifact_by_id(10, 8, json!({"from": "A", "to": "C"})).await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    #[tokio::test]
    async fn update_artifact_by_id_rejects_file_path_change() {
        let mut repo = MockArtifactRepository::new();
        let artifact = fake_artifact(9, 10, ArtifactKind::Plan, json!({"path": ".insights/plan.md"}));
        let a = artifact.clone();
        repo.expect_find_by_id().returning(move |_, _| {
            let a = a.clone();
            Box::pin(async move { Ok(Some(a)) })
        });
        let svc = make_svc(repo);
        let result = svc.update_artifact_by_id(10, 9, json!({"path": ".insights/plan-v2.md"})).await;
        assert!(matches!(result, Err(Error::Validation(_))));
    }

    // remove_artifact_by_id issue_id mismatch

    #[tokio::test]
    async fn remove_artifact_by_id_issue_id_mismatch_returns_not_found() {
        let mut repo = MockArtifactRepository::new();
        // artifact belongs to issue 99, but we pass issue_id=10
        let artifact = fake_artifact(11, 99, ArtifactKind::Comment, json!({"text": "hi"}));
        let a = artifact.clone();
        repo.expect_find_by_id().returning(move |_, _| {
            let a = a.clone();
            Box::pin(async move { Ok(Some(a)) })
        });
        let svc = make_svc(repo);
        let result = svc.remove_artifact_by_id(10, 11).await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    // add_artifact string-body coercion: the persisted body must be the parsed
    // object, not the raw JSON-encoded string.

    fn mock_repo_echoing_create() -> MockArtifactRepository {
        let mut repo = MockArtifactRepository::new();
        repo.expect_create().returning(|_, record| {
            let slug = record.slug.clone();
            let kind = record.kind.clone();
            let issue_id = record.issue_id;
            let body = record.body.clone();
            let created_by = record.created_by.clone();
            Box::pin(async move {
                Ok(IssueArtifact {
                    id: 1,
                    token: ArtifactToken::new(1),
                    issue_id,
                    kind,
                    slug,
                    body,
                    created_by,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            })
        });
        repo
    }

    #[tokio::test]
    async fn add_artifact_coerces_string_body_for_triage_result() {
        let svc = make_svc(mock_repo_echoing_create());
        let artifact = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::TriageResult,
                slug: None,
                body: Value::String(r#"{"path":"x.md"}"#.to_string()),
                created_by: "U_1".into(),
            })
            .await
            .unwrap();
        assert_eq!(artifact.body, json!({"path": "x.md"}));
    }

    #[tokio::test]
    async fn add_artifact_coerces_string_body_for_research() {
        let svc = make_svc(mock_repo_echoing_create());
        let artifact = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::Research,
                slug: Some("topic-a".into()),
                body: Value::String(r#"{"topic_token":"T_1","status":"completed","path":"r.md"}"#.to_string()),
                created_by: "U_1".into(),
            })
            .await
            .unwrap();
        assert_eq!(artifact.body, json!({"topic_token": "T_1", "status": "completed", "path": "r.md"}));
    }

    #[tokio::test]
    async fn add_artifact_coerces_string_body_for_comment() {
        let svc = make_svc(mock_repo_echoing_create());
        let artifact = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::Comment,
                slug: Some("note".into()),
                body: Value::String(r#"{"text":"hi"}"#.to_string()),
                created_by: "U_1".into(),
            })
            .await
            .unwrap();
        assert_eq!(artifact.body, json!({"text": "hi"}));
    }

    #[tokio::test]
    async fn add_artifact_object_body_unchanged() {
        let svc = make_svc(mock_repo_echoing_create());
        let artifact = svc
            .add_artifact(NewArtifact {
                issue_id: 1,
                kind: ArtifactKind::TriageResult,
                slug: None,
                body: json!({"path": "x.md"}),
                created_by: "U_1".into(),
            })
            .await
            .unwrap();
        assert_eq!(artifact.body, json!({"path": "x.md"}));
    }

    #[tokio::test]
    async fn update_artifact_coerces_string_body() {
        let mut repo = MockArtifactRepository::new();
        let existing = fake_artifact(20, 10, ArtifactKind::Comment, json!({"text": "old"}));
        let a = existing.clone();
        repo.expect_find_by_slug().returning(move |_, _, _| {
            let a = a.clone();
            Box::pin(async move { Ok(Some(a)) })
        });
        repo.expect_update().returning(|_, a| Box::pin(async move { Ok(a) }));
        let svc = make_svc(repo);
        let artifact = svc.update_artifact(10, "note", Value::String(r#"{"text":"new"}"#.to_string())).await.unwrap();
        assert_eq!(artifact.body, json!({"text": "new"}));
    }
}
