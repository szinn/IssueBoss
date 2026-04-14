use ib_core::issue::{IssueFilter, IssuePriority, IssueSize, IssueStatus};

use crate::{fixtures, setup};

/// Issues within a project are numbered sequentially starting from 1, and
/// the slug is derived from the project prefix and number.
#[tokio::test]
async fn issue_numbers_are_sequential() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Sequenced", "sequenced", "SEQ").await;
    let i1 = fixtures::create_issue(&ctx.services, project.id, "SEQ", "First").await;
    let i2 = fixtures::create_issue(&ctx.services, project.id, "SEQ", "Second").await;
    let i3 = fixtures::create_issue(&ctx.services, project.id, "SEQ", "Third").await;
    assert_eq!(i1.slug, "SEQ-1");
    assert_eq!(i2.slug, "SEQ-2");
    assert_eq!(i3.slug, "SEQ-3");
}

/// Each project maintains its own independent issue number sequence; a new
/// project always starts at 1 regardless of issues in other projects.
#[tokio::test]
async fn issue_numbers_restart_per_project() {
    let ctx = setup().await;
    let proj_a = fixtures::create_project(&ctx.services, "Alpha", "alpha-seq", "ALPH").await;
    let proj_b = fixtures::create_project(&ctx.services, "Beta", "beta-seq", "BETA").await;

    // Create an issue in alpha first to advance its counter.
    let a1 = fixtures::create_issue(&ctx.services, proj_a.id, "ALPH", "Alpha issue one").await;
    let a2 = fixtures::create_issue(&ctx.services, proj_a.id, "ALPH", "Alpha issue two").await;

    // Beta starts fresh at 1.
    let b1 = fixtures::create_issue(&ctx.services, proj_b.id, "BETA", "Beta issue one").await;

    assert_eq!(a1.slug, "ALPH-1");
    assert_eq!(a2.slug, "ALPH-2");
    assert_eq!(b1.slug, "BETA-1");
}

#[tokio::test]
async fn issue_update_fields() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Update", "issue-update", "UPD").await;
    let issue = fixtures::create_issue(&ctx.services, project.id, "UPD", "Original title").await;

    let mut updated = issue.clone();
    updated.title = "New title".to_string();
    updated.description = "Better description".to_string();
    updated.priority = IssuePriority::High;
    updated.size = Some(IssueSize::Medium);

    let saved = ctx.services.issue_service().update_issue(updated).await.unwrap();
    assert_eq!(saved.title, "New title");
    assert_eq!(saved.description, "Better description");
    assert_eq!(saved.priority, IssuePriority::High);
    assert_eq!(saved.size, Some(IssueSize::Medium));
}

#[tokio::test]
async fn issue_transition_triage_in_progress() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Trans", "issue-trans", "TR").await;
    let issue = fixtures::create_issue(&ctx.services, project.id, "TR", "Work item").await;
    assert_eq!(issue.status, IssueStatus::TriageNeeded);

    let issue = ctx
        .services
        .issue_service()
        .transition_issue(issue.token, IssueStatus::TriageInProgress, None, Some(0))
        .await
        .unwrap();
    assert_eq!(issue.status, IssueStatus::TriageInProgress);
}

#[tokio::test]
async fn issue_list_filter_by_status() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Filter", "issue-filter-status", "FS").await;
    let i1 = fixtures::create_issue(&ctx.services, project.id, "FS", "In progress one").await;
    let _i2 = fixtures::create_issue(&ctx.services, project.id, "FS", "Still triage needed").await;

    ctx.services
        .issue_service()
        .transition_issue(i1.token, IssueStatus::TriageInProgress, None, Some(0))
        .await
        .unwrap();

    let filter = IssueFilter {
        status: Some(IssueStatus::TriageInProgress),
        ..Default::default()
    };
    let results = ctx.services.issue_service().list_issues(project.id, filter).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slug, "FS-1");
}

#[tokio::test]
async fn issue_list_filter_by_priority() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "PriFilter", "issue-filter-pri", "FP").await;

    let i1 = fixtures::create_issue(&ctx.services, project.id, "FP", "High prio").await;
    let _i2 = fixtures::create_issue(&ctx.services, project.id, "FP", "Medium prio").await;
    let mut high = i1.clone();
    high.priority = IssuePriority::High;
    ctx.services.issue_service().update_issue(high).await.unwrap();

    let filter = IssueFilter {
        priority: Some(IssuePriority::High),
        ..Default::default()
    };
    let results = ctx.services.issue_service().list_issues(project.id, filter).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slug, "FP-1");
}

#[tokio::test]
#[allow(clippy::similar_names)]
async fn issue_list_exclude_blocked() {
    use ib_core::relationship::RelationshipKind;

    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Blocked", "issue-blocked", "BL").await;
    let blocker = fixtures::create_issue(&ctx.services, project.id, "BL", "Blocker").await;
    let blocked = fixtures::create_issue(&ctx.services, project.id, "BL", "Blocked").await;

    ctx.services
        .relationship_service()
        .add_relationship(&blocked.slug, &blocker.slug, RelationshipKind::DependsOn)
        .await
        .unwrap();

    let filter = IssueFilter {
        exclude_blocked: Some(true),
        ..Default::default()
    };
    let results = ctx.services.issue_service().list_issues(project.id, filter).await.unwrap();
    let slugs: Vec<_> = results.iter().map(|i| i.slug.as_str()).collect();
    assert!(slugs.contains(&"BL-1"), "blocker should appear");
    assert!(!slugs.contains(&"BL-2"), "blocked should be excluded");
}

#[tokio::test]
async fn issue_find_by_slug_not_found() {
    let ctx = setup().await;
    let result = ctx.services.issue_service().find_by_slug("NOPE-999").await.unwrap();
    assert!(result.is_none());
}
