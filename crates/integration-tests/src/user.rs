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

#[tokio::test]
async fn find_by_username_returns_user() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "charlie").await;

    let found = ctx.services.user_service().find_by_username("charlie").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, user.id);
}

#[tokio::test]
async fn find_by_username_returns_none_for_unknown() {
    let ctx = setup().await;

    let found = ctx.services.user_service().find_by_username("nobody").await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn update_user_conflict_detected() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "diana").await;

    // Pass a User with a wrong version number to trigger the conflict check.
    let mut wrong_version = user.clone();
    wrong_version.version = 999;
    wrong_version.full_name = "Diana Conflict".to_owned();

    let err = ctx.services.user_service().update_user(wrong_version).await.unwrap_err();
    assert!(
        matches!(err, ib_core::Error::RepositoryError(ib_core::RepositoryError::Conflict)),
        "expected Conflict, got {err:?}"
    );
}

#[tokio::test]
async fn delete_user_returns_deleted_user() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "eve").await;
    let id = user.id;

    let deleted = ctx.services.user_service().delete_user(id).await.unwrap();
    assert_eq!(deleted.id, id);

    let found = ctx.services.user_service().find_by_id(id).await.unwrap();
    assert!(found.is_none());
}

#[tokio::test]
async fn update_user_fields() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "update-me").await;
    let mut updated = user.clone();
    updated.full_name = "Updated Name".to_string();

    let saved = ctx.services.user_service().update_user(updated).await.unwrap();
    assert_eq!(saved.full_name, "Updated Name");
}

#[tokio::test]
async fn list_users_returns_all() {
    let ctx = setup().await;
    fixtures::create_user(&ctx.services, "list-user-one").await;
    fixtures::create_user(&ctx.services, "list-user-two").await;

    let users = ctx.services.user_service().list_users().await.unwrap();
    assert!(users.len() >= 2, "expected at least 2 users");
    let names: Vec<_> = users.iter().map(|u| u.username.as_str()).collect();
    assert!(names.contains(&"list-user-one"));
    assert!(names.contains(&"list-user-two"));
}
