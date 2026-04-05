pub use sea_orm_migration::prelude::*;

mod m20260405_000001_create_users;

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20260405_000001_create_users::Migration)]
    }
}
