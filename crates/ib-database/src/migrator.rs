use sea_orm_migration::prelude::*;

use crate::migration::CreateUsers;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(CreateUsers)]
    }
}
