mod api_key;
mod artifact;
mod context;
mod fixtures;
mod issue;
mod project;
mod project_member;
mod relationship;
mod user;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "sqlite")]
#[cfg_attr(
    feature = "postgres",
    allow(dead_code, reason = "postgres takes priority; other backends are inactive when it is enabled")
)]
mod sqlite;

// Priority: postgres > mysql > sqlite when multiple features are enabled.
#[cfg(feature = "postgres")]
pub(crate) use postgres::setup;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
pub(crate) use sqlite::setup;

#[cfg(not(any(feature = "postgres", feature = "sqlite")))]
compile_error!("At least one database backend feature must be enabled: postgres, sqlite");

#[tokio::test]
async fn test_setup() {
    let _ctx = setup().await;
}
