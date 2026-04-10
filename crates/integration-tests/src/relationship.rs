use ib_core::{Error, relationship::RelationshipKind};

use crate::{fixtures, setup};

#[tokio::test]
async fn add_relationship_depends_on() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Rel", "rel-dep", "RD").await;
    let a = fixtures::create_issue(&ctx.services, project.id, "RD", "Issue A").await;
    let b = fixtures::create_issue(&ctx.services, project.id, "RD", "Issue B").await;

    let rel = ctx
        .services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .expect("add DependsOn failed");

    assert_eq!(rel.from_issue_id, a.id);
    assert_eq!(rel.to_issue_id, b.id);
    assert_eq!(rel.kind, RelationshipKind::DependsOn);

    // From A's view: A depends_on B
    let a_rels = ctx.services.relationship_service().list_for_issue(a.id).await.unwrap();
    assert_eq!(a_rels.depends_on.len(), 1);
    assert_eq!(a_rels.depends_on[0].slug, b.slug);
    assert!(a_rels.blocks.is_empty());

    // From B's view: B blocks A
    let b_rels = ctx.services.relationship_service().list_for_issue(b.id).await.unwrap();
    assert_eq!(b_rels.blocks.len(), 1);
    assert_eq!(b_rels.blocks[0].slug, a.slug);
    assert!(b_rels.depends_on.is_empty());
}

#[tokio::test]
async fn add_relationship_related_to_is_symmetric() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "RelSym", "rel-sym", "RS").await;
    let a = fixtures::create_issue(&ctx.services, project.id, "RS", "Issue A").await;
    let b = fixtures::create_issue(&ctx.services, project.id, "RS", "Issue B").await;

    ctx.services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::RelatedTo)
        .await
        .expect("add RelatedTo failed");

    let a_rels = ctx.services.relationship_service().list_for_issue(a.id).await.unwrap();
    assert_eq!(a_rels.related_to.len(), 1);
    assert_eq!(a_rels.related_to[0].slug, b.slug);

    let b_rels = ctx.services.relationship_service().list_for_issue(b.id).await.unwrap();
    assert_eq!(b_rels.related_to.len(), 1);
    assert_eq!(b_rels.related_to[0].slug, a.slug);
}

#[tokio::test]
async fn cycle_detection_returns_error() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Cycle", "rel-cycle", "CY").await;
    let a = fixtures::create_issue(&ctx.services, project.id, "CY", "A").await;
    let b = fixtures::create_issue(&ctx.services, project.id, "CY", "B").await;
    let c = fixtures::create_issue(&ctx.services, project.id, "CY", "C").await;

    // A→B, B→C established
    ctx.services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .unwrap();
    ctx.services
        .relationship_service()
        .add_relationship(&b.slug, &c.slug, RelationshipKind::DependsOn)
        .await
        .unwrap();

    // C→A would create a cycle
    let err = ctx
        .services
        .relationship_service()
        .add_relationship(&c.slug, &a.slug, RelationshipKind::DependsOn)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::CycleDetected), "expected CycleDetected, got {err:?}");

    // Direct cycle: B→A
    let err2 = ctx
        .services
        .relationship_service()
        .add_relationship(&b.slug, &a.slug, RelationshipKind::DependsOn)
        .await
        .unwrap_err();
    assert!(matches!(err2, Error::CycleDetected), "expected CycleDetected, got {err2:?}");
}

#[tokio::test]
async fn remove_relationship_removes_correct_kind() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "RelRm", "rel-rm", "RM").await;
    let a = fixtures::create_issue(&ctx.services, project.id, "RM", "A").await;
    let b = fixtures::create_issue(&ctx.services, project.id, "RM", "B").await;

    ctx.services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .unwrap();
    ctx.services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::RelatedTo)
        .await
        .unwrap();

    // Remove only DependsOn
    let removed = ctx
        .services
        .relationship_service()
        .remove_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .unwrap();
    assert!(removed);

    let rels = ctx.services.relationship_service().list_for_issue(a.id).await.unwrap();
    assert!(rels.depends_on.is_empty(), "DependsOn should be gone");
    assert_eq!(rels.related_to.len(), 1, "RelatedTo should remain");
}

#[tokio::test]
async fn remove_related_to_works_from_either_direction() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "RelDir", "rel-rt", "RT").await;
    let a = fixtures::create_issue(&ctx.services, project.id, "RT", "A").await;
    let b = fixtures::create_issue(&ctx.services, project.id, "RT", "B").await;

    // Add as A→B
    ctx.services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::RelatedTo)
        .await
        .unwrap();

    // Remove as B→A (reverse direction)
    let removed = ctx
        .services
        .relationship_service()
        .remove_relationship(&b.slug, &a.slug, RelationshipKind::RelatedTo)
        .await
        .unwrap();
    assert!(removed);

    let rels = ctx.services.relationship_service().list_for_issue(a.id).await.unwrap();
    assert!(rels.related_to.is_empty());
}

#[tokio::test]
async fn relationship_add_and_list_smoke_test() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Cascade", "rel-cascade", "CC").await;
    let a = fixtures::create_issue(&ctx.services, project.id, "CC", "A").await;
    let b = fixtures::create_issue(&ctx.services, project.id, "CC", "B").await;

    ctx.services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .unwrap();
    let rels = ctx.services.relationship_service().list_for_issue(a.id).await.unwrap();
    assert_eq!(rels.depends_on.len(), 1);
}

#[tokio::test]
async fn cross_project_relationship_rejected() {
    let ctx = setup().await;
    let proj_a = fixtures::create_project(&ctx.services, "ProjA", "rel-pa", "PA").await;
    let proj_b = fixtures::create_project(&ctx.services, "ProjB", "rel-pb", "PB").await;
    let a = fixtures::create_issue(&ctx.services, proj_a.id, "PA", "A").await;
    let b = fixtures::create_issue(&ctx.services, proj_b.id, "PB", "B").await;

    let err = ctx
        .services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Validation(_)), "expected Validation, got {err:?}");
}

#[tokio::test]
async fn duplicate_relationship_rejected() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "Dup", "rel-dup", "DP").await;
    let a = fixtures::create_issue(&ctx.services, project.id, "DP", "A").await;
    let b = fixtures::create_issue(&ctx.services, project.id, "DP", "B").await;

    ctx.services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .unwrap();
    let err = ctx
        .services
        .relationship_service()
        .add_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::AlreadyExists), "expected AlreadyExists, got {err:?}");
}

#[tokio::test]
async fn remove_nonexistent_relationship_returns_false() {
    let ctx = setup().await;
    let project = fixtures::create_project(&ctx.services, "RmNE", "rel-rmne", "NE").await;
    let a = fixtures::create_issue(&ctx.services, project.id, "NE", "A").await;
    let b = fixtures::create_issue(&ctx.services, project.id, "NE", "B").await;

    let removed = ctx
        .services
        .relationship_service()
        .remove_relationship(&a.slug, &b.slug, RelationshipKind::DependsOn)
        .await
        .unwrap();
    assert!(!removed, "removing a non-existent relationship should return false");
}
