use ib_core::{
    Error,
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

    // Add TriageResult — gate passes
    let _triage = ctx
        .services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::TriageResult,
            body: serde_json::json!({"path": "insights/triage/art-1.md"}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();

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

    // Add a topic
    let topic = ctx
        .services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::ResearchTopic,
            body: serde_json::json!({"description": "Investigate auth flow"}),
            created_by: "U_test".into(),
        })
        .await
        .unwrap();

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

    // Cover the topic
    ctx.services
        .artifact_service()
        .add_artifact(NewArtifact {
            issue_id: issue.id,
            kind: ArtifactKind::Research,
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
