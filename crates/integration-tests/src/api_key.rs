use crate::{fixtures, setup};

#[tokio::test]
async fn create_and_list_for_user() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "alice").await;
    fixtures::create_api_key(&ctx.services, user.id).await;
    fixtures::create_api_key(&ctx.services, user.id).await;

    let keys = ctx.services.api_key_service().list_for_user(user.id).await.unwrap();
    assert_eq!(keys.len(), 2);
}

#[tokio::test]
async fn revoke_key_removes_it() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "bob").await;
    let key = fixtures::create_api_key(&ctx.services, user.id).await;

    ctx.services.api_key_service().revoke_key(key.id).await.unwrap();

    let remaining = ctx.services.api_key_service().list_for_user(user.id).await.unwrap();
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn record_usage_sets_last_used_at() {
    let ctx = setup().await;
    let user = fixtures::create_user(&ctx.services, "carol").await;
    let key = fixtures::create_api_key(&ctx.services, user.id).await;
    assert!(key.last_used_at.is_none());

    ctx.services.api_key_service().record_usage(key.clone()).await.unwrap();

    let updated = ctx
        .services
        .api_key_service()
        .find_by_hash(&key.key_hash)
        .await
        .unwrap()
        .expect("key should still exist");
    assert!(updated.last_used_at.is_some());
}

#[tokio::test]
async fn list_for_user_isolates_by_user() {
    let ctx = setup().await;
    let user_a = fixtures::create_user(&ctx.services, "dave").await;
    let user_b = fixtures::create_user(&ctx.services, "eve").await;
    fixtures::create_api_key(&ctx.services, user_a.id).await;
    fixtures::create_api_key(&ctx.services, user_a.id).await;
    fixtures::create_api_key(&ctx.services, user_b.id).await;

    let keys_a = ctx.services.api_key_service().list_for_user(user_a.id).await.unwrap();
    let keys_b = ctx.services.api_key_service().list_for_user(user_b.id).await.unwrap();
    assert_eq!(keys_a.len(), 2);
    assert_eq!(keys_b.len(), 1);
}
