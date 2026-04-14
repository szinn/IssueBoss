use sea_orm_migration::{
    prelude::*,
    schema::{big_integer, string, timestamp_with_time_zone},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProjectMembers::Table)
                    .if_not_exists()
                    .col(big_integer(ProjectMembers::ProjectId).not_null())
                    .col(big_integer(ProjectMembers::UserId).not_null())
                    .col(string(ProjectMembers::Capabilities).not_null().default("[]"))
                    .col(timestamp_with_time_zone(ProjectMembers::CreatedAt))
                    .col(timestamp_with_time_zone(ProjectMembers::UpdatedAt))
                    .primary_key(Index::create().col(ProjectMembers::ProjectId).col(ProjectMembers::UserId))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_members_project_id")
                            .from(ProjectMembers::Table, ProjectMembers::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_project_members_user_id")
                            .from(ProjectMembers::Table, ProjectMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // This project uses forward-only migrations. Rollback is not supported;
        // down() is intentionally left as a no-op. To undo a migration, create
        // a new migration that reverses the change.
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ProjectMembers {
    Table,
    ProjectId,
    UserId,
    Capabilities,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
