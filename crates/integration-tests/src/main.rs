use crate::context::TestContext;

mod api_key;
mod artifact;
mod context;
mod fixtures;
mod grpc_auth;
mod grpc_context;
mod issue;
mod project;
mod project_member;
mod relationship;
mod user;

#[cfg(feature = "postgres")]
mod postgres;
#[cfg(all(feature = "sqlite", not(feature = "postgres")))]
mod sqlite;

#[cfg(not(any(feature = "postgres", feature = "sqlite")))]
compile_error!("At least one database backend feature must be enabled: postgres, sqlite");

pub(crate) async fn setup() -> TestContext {
    #[cfg(feature = "postgres")]
    return postgres::setup().await;
    #[cfg(all(feature = "sqlite", not(feature = "postgres")))]
    return sqlite::setup().await;
    #[cfg(not(any(feature = "postgres", feature = "sqlite")))]
    unreachable!()
}

#[tokio::test]
async fn test_setup() {
    let _ctx = setup().await;
}
