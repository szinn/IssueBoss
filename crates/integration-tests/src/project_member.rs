use ib_core::{
    project::NewProjectMember,
    types::{Capabilities, Capability},
};

use crate::{fixtures, setup};

#[tokio::test]
async fn update_member_capabilities() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "alice").await;
    let project = fixtures::create_project(&ctx.services, "P", "proj-u", "PU").await;
    let member = fixtures::add_project_member(&ctx.services, project.id, user.id).await;
    assert!(member.capabilities.0.is_empty());

    let updated_member = ib_core::project::ProjectMember {
        capabilities: Capabilities(vec![Capability::ViewIssues]),
        ..member
    };
    let updated = ctx.services.project_service().update_member(updated_member).await.unwrap();

    assert!(!updated.capabilities.0.is_empty());
    assert!(updated.capabilities.0.contains(&Capability::ViewIssues));
}

#[tokio::test]
async fn delete_member_removes_from_project() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "bob").await;
    let project = fixtures::create_project(&ctx.services, "P2", "proj-d", "PD").await;
    fixtures::add_project_member(&ctx.services, project.id, user.id).await;

    ctx.services.project_service().remove_member(project.id, user.id).await.unwrap();

    let members = ctx.services.project_service().list_members(project.id).await.unwrap();
    assert!(members.is_empty());
}

#[tokio::test]
async fn add_duplicate_member_returns_error() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "carol").await;
    let project = fixtures::create_project(&ctx.services, "P3", "proj-dup", "PDUP").await;
    fixtures::add_project_member(&ctx.services, project.id, user.id).await;

    let result = ctx
        .services
        .project_service()
        .add_member(NewProjectMember {
            project_id: project.id,
            user_id: user.id,
            capabilities: Capabilities::default(),
        })
        .await;

    assert!(result.is_err(), "adding a duplicate member should fail");
}
