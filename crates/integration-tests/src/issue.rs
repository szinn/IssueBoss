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
