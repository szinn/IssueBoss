pub use sea_orm_migration::prelude::*;

mod m20260405_000001_create_users;
mod m20260406_000002_create_api_keys;
mod m20260407_000003_create_projects;
mod m20260407_000004_create_project_members;
mod m20260407_000005_create_issues;
mod m20260408_000006_create_issue_artifacts;
mod m20260409_000007_add_artifact_slug;

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260405_000001_create_users::Migration),
            Box::new(m20260406_000002_create_api_keys::Migration),
            Box::new(m20260407_000003_create_projects::Migration),
            Box::new(m20260407_000004_create_project_members::Migration),
            Box::new(m20260407_000005_create_issues::Migration),
            Box::new(m20260408_000006_create_issue_artifacts::Migration),
            Box::new(m20260409_000007_add_artifact_slug::Migration),
        ]
    }
}
