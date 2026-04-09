use ib_core::{
    Error, RepositoryError,
    artifact::{ArtifactKind, NewArtifact},
    issue::IssueStatus,
};

use crate::{fixtures, setup};

#[tokio::test]
async fn artifact_lifecycle() {
    let ctx = setup().await;

    // Setup: project + issue
    let project = fixtures::create_project(&ctx.services, "Artifact Test", "artifact-test", "ART").await;
    let issue = fixtures::create_issue(&ctx.services, project.id, "ART", "Test issue").await;
    assert_eq!(issue.status, IssueStatus::Triage);

    // Gate: Triage → SpecNeeded blocked without TriageResult
    let err = ctx.services.issue_service().transition_issue(issue.token, IssueStatus::SpecNeeded, None).await;
    assert!(matches!(err, Err(Error::GateFailure { ref condition, .. }) if condition == "missing_triage_result"));

    // Add TriageResult — gate passes (slug auto-assigned by service)
    let _triage = ctx
        .services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::TriageResult,
            slug: None,
            body: serde_json::json!({"path": "insights/triage/art-1.md"}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();
    assert_eq!(_triage.slug, Some("triage".to_string()));

    let issue = ctx
        .services
        .issue_service()
        .transition_issue(issue.token, IssueStatus::SpecNeeded, Some("Triage done".into()))
        .await
        .unwrap();
    assert_eq!(issue.status, IssueStatus::SpecNeeded);

    // StatusTransition artifact auto-created
    let transitions = ctx
        .services
        .artifact_service()
        .list_artifacts(issue.id, Some(vec![ArtifactKind::StatusTransition]), false)
        .await
        .unwrap();
    assert_eq!(transitions.len(), 1);
    assert_eq!(transitions[0].body["from"], "Triage");
    assert_eq!(transitions[0].body["to"], "SpecNeeded");
    assert_eq!(transitions[0].body["reason"], "Triage done");

    // Human-driven: SpecNeeded → ResearchNeeded (no gate)
    let issue = ctx
        .services
        .issue_service()
        .transition_issue(issue.token, IssueStatus::ResearchNeeded, None)
        .await
        .unwrap();

    // Gate: no topics → blocked
    let err = ctx
        .services
        .issue_service()
        .transition_issue(issue.token, IssueStatus::ResearchInProgress, None)
        .await;
    assert!(matches!(err, Err(Error::GateFailure { ref condition, .. }) if condition == "no_research_topics"));

    // Add a topic with explicit slug
    let topic = ctx
        .services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::ResearchTopic,
            slug: Some("investigate-auth-flow".into()),
            body: serde_json::json!({"description": "Investigate auth flow"}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();
    assert_eq!(topic.slug, Some("investigate-auth-flow".to_string()));

    let issue = ctx
        .services
        .issue_service()
        .transition_issue(issue.token, IssueStatus::ResearchInProgress, None)
        .await
        .unwrap();

    // Gate: uncovered topic blocks ResearchInProgress → ResearchInReview
    let err = ctx
        .services
        .issue_service()
        .transition_issue(issue.token, IssueStatus::ResearchInReview, None)
        .await;
    assert!(matches!(err, Err(Error::GateFailure { ref condition, .. }) if condition == "uncovered_research_topics"));

    // Cover the topic with explicit slug
    ctx.services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::Research,
            slug: Some("auth-flow-research".into()),
            body: serde_json::json!({
                "topic_token": topic.token.to_string(),
                "status": "completed",
                "path": "insights/research/art-1-auth.md"
            }),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();

    // uncovered_only now empty
    let uncovered = ctx
        .services
        .artifact_service()
        .list_artifacts(issue.id, Some(vec![ArtifactKind::ResearchTopic]), true)
        .await
        .unwrap();
    assert!(uncovered.is_empty());

    // Gate passes
    let issue = ctx
        .services
        .issue_service()
        .transition_issue(issue.token, IssueStatus::ResearchInReview, None)
        .await
        .unwrap();
    assert_eq!(issue.status, IssueStatus::ResearchInReview);
}

#[tokio::test]
async fn artifact_slug_update() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Slug Update", "slug-update", "SU").await;
    let issue = fixtures::create_issue(&ctx.services, project.id, "SU", "Update test").await;

    ctx.services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::Comment,
            slug: Some("initial-note".into()),
            body: serde_json::json!({"text": "original text"}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();

    let updated = ctx
        .services
        .artifact_service()
        .update_artifact(issue.id, "initial-note", serde_json::json!({"text": "revised text"}))
        .await
        .unwrap();

    assert_eq!(updated.body["text"], "revised text");
    assert_eq!(updated.slug, Some("initial-note".to_string()));
}

#[tokio::test]
async fn artifact_slug_remove() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Slug Remove", "slug-remove", "SR").await;
    let issue = fixtures::create_issue(&ctx.services, project.id, "SR", "Remove test").await;

    ctx.services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::Comment,
            slug: Some("to-remove".into()),
            body: serde_json::json!({"text": "remove me"}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();

    ctx.services.artifact_service().remove_artifact(issue.id, "to-remove").await.unwrap();

    let result = ctx
        .services
        .artifact_service()
        .update_artifact(issue.id, "to-remove", serde_json::json!({"text": "ghost"}))
        .await;
    assert!(matches!(result, Err(ib_core::Error::RepositoryError(RepositoryError::NotFound))));
}

#[tokio::test]
async fn artifact_slug_uniqueness_enforced() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Slug Unique", "slug-unique", "SL").await;
    let issue = fixtures::create_issue(&ctx.services, project.id, "SL", "Uniqueness test").await;

    ctx.services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::Comment,
            slug: Some("dupe-slug".into()),
            body: serde_json::json!({"text": "first"}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();

    let result = ctx
        .services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::Comment,
            slug: Some("dupe-slug".into()),
            body: serde_json::json!({"text": "second"}),
            created_by: "U_test".into(),
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn move_artifact_updates_matching_artifacts() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Move Test", "move-test", "MV").await;

    let issue1 = fixtures::create_issue(&ctx.services, project.id, "MV", "Issue 1").await;
    let issue2 = fixtures::create_issue(&ctx.services, project.id, "MV", "Issue 2").await;

    let old_path = ".insights/specs/shared-spec.md";
    let new_path = ".insights/specs/moved-spec.md";

    // Add TriageResult (singleton) pointing to old_path on both issues
    ctx.services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue1.id,
            kind: ArtifactKind::TriageResult,
            slug: None,
            body: serde_json::json!({"path": old_path}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();

    ctx.services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue2.id,
            kind: ArtifactKind::TriageResult,
            slug: None,
            body: serde_json::json!({"path": old_path}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();

    // Move the path — should update both artifacts
    let updated = ctx.services.artifact_service().move_artifact(old_path, new_path).await.unwrap();
    assert_eq!(updated.len(), 2);
    for artifact in &updated {
        assert_eq!(artifact.body["path"], new_path);
    }

    // Verify via list_artifacts that both DB records reflect the new path
    let arts1 = ctx
        .services
        .artifact_service()
        .list_artifacts(issue1.id, Some(vec![ArtifactKind::TriageResult]), false)
        .await
        .unwrap();
    assert_eq!(arts1[0].body["path"], new_path);

    let arts2 = ctx
        .services
        .artifact_service()
        .list_artifacts(issue2.id, Some(vec![ArtifactKind::TriageResult]), false)
        .await
        .unwrap();
    assert_eq!(arts2[0].body["path"], new_path);
}

#[tokio::test]
async fn move_artifact_noop_same_path() {
    let ctx = setup().await;
    let result = ctx
        .services
        .artifact_service()
        .move_artifact(".insights/spec.md", ".insights/spec.md")
        .await
        .unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn move_artifact_no_matching_artifacts() {
    let ctx = setup().await;
    let result = ctx
        .services
        .artifact_service()
        .move_artifact(".insights/nonexistent.md", ".insights/also-nonexistent.md")
        .await
        .unwrap();
    assert!(result.is_empty());
}
