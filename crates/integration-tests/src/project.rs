use crate::{fixtures, setup};

/// Deleting a project removes all its issues via the database cascade on
/// `issues.project_id`.
#[tokio::test]
async fn delete_project_removes_issues() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Alpha", "alpha", "ALPH").await;
    let issue = fixtures::create_issue(&ctx.services, project.id, "ALPH", "First issue").await;

    ctx.services.project_service().delete_project(project.id).await.unwrap();

    let found = ctx.services.issue_service().find_by_slug(&issue.slug).await.unwrap();
    assert!(found.is_none(), "issues should be removed when project is deleted");
}

/// Deleting a project removes all its project memberships via the database
/// cascade on `project_members.project_id`.
#[tokio::test]
async fn delete_project_removes_members() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "carol").await;
    let project = fixtures::create_project(&ctx.services, "Beta", "beta", "BETA").await;
    fixtures::add_project_member(&ctx.services, project.id, user.id).await;

    ctx.services.project_service().delete_project(project.id).await.unwrap();

    let members = ctx.services.project_service().list_members(project.id).await.unwrap();
    assert!(members.is_empty(), "project memberships should be removed when project is deleted");
}

#[tokio::test]
async fn update_project_fields() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Original", "proj-update", "PU").await;
    let mut updated = project.clone();
    updated.name = "Renamed".to_string();

    let saved = ctx.services.project_service().update_project(updated).await.unwrap();
    assert_eq!(saved.name, "Renamed");
}

#[tokio::test]
async fn list_projects_returns_all() {
    let ctx = setup().await;
    fixtures::create_project(&ctx.services, "ListOne", "list-one", "LO").await;
    fixtures::create_project(&ctx.services, "ListTwo", "list-two", "LT").await;

    let projects = ctx.services.project_service().list_projects().await.unwrap();
    assert!(projects.len() >= 2, "expected at least 2 projects");
    let names: Vec<_> = projects.iter().map(|p| p.name.as_str()).collect();
    assert!(names.contains(&"ListOne"));
    assert!(names.contains(&"ListTwo"));
}

#[tokio::test]
async fn project_member_add_list_remove() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "member-user").await;
    let project = fixtures::create_project(&ctx.services, "MemberTest", "member-proj", "MP").await;

    fixtures::add_project_member(&ctx.services, project.id, user.id).await;

    let members = ctx.services.project_service().list_members(project.id).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].user_id, user.id);

    ctx.services.project_service().remove_member(project.id, user.id).await.unwrap();

    let members = ctx.services.project_service().list_members(project.id).await.unwrap();
    assert!(members.is_empty(), "member should be removed");
}
