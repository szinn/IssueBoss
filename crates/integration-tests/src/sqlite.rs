use ib_database::create_repository_service;
use sea_orm::Database;

use crate::context::TestContext;

pub async fn setup() -> TestContext {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let repository_service = create_repository_service(db).await.unwrap();
    let core_services = ib_core::create_services(repository_service.clone());

    TestContext::new(core_services, ())
}
