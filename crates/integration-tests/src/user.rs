use crate::{fixtures, setup};

/// Deleting a user removes all API keys associated with that user via the
/// database cascade on `api_keys.user_id`.
#[tokio::test]
async fn delete_user_removes_api_keys() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "alice").await;
    fixtures::create_api_key(&ctx.services, user.id).await;
    fixtures::create_api_key(&ctx.services, user.id).await;

    ctx.services.user_service().delete_user(user.id).await.unwrap();

    let keys = ctx.services.api_key_service().list_for_user(user.id).await.unwrap();
    assert!(keys.is_empty(), "api keys should be removed when user is deleted");
}

/// Deleting a user removes their membership from all projects via the
/// database cascade on `project_members.user_id`.
#[tokio::test]
async fn delete_user_removes_project_memberships() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "bob").await;
    let project = fixtures::create_project(&ctx.services, "Test Project", "test-project", "TEST").await;
    fixtures::add_project_member(&ctx.services, project.id, user.id).await;

    ctx.services.user_service().delete_user(user.id).await.unwrap();

    let members = ctx.services.project_service().list_members(project.id).await.unwrap();
    assert!(members.is_empty(), "project memberships should be removed when user is deleted");
}
