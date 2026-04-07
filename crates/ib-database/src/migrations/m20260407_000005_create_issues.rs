use sea_orm_migration::{
    prelude::*,
    schema::{big_integer, integer, string, text, timestamp_with_time_zone},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Issues::Table)
                    .if_not_exists()
                    .col(big_integer(Issues::Id).not_null().primary_key())
                    .col(string(Issues::Token).not_null().unique_key())
                    .col(big_integer(Issues::ProjectId).not_null())
                    .col(integer(Issues::Number).not_null())
                    .col(string(Issues::Title).not_null())
                    .col(text(Issues::Description).not_null().default(""))
                    .col(string(Issues::Status).not_null())
                    .col(string(Issues::Priority).not_null())
                    .col(string(Issues::Size).null())
                    .col(string(Issues::Slug).not_null().unique_key())
                    .col(big_integer(Issues::Version).not_null().default(0))
                    .col(timestamp_with_time_zone(Issues::CreatedAt))
                    .col(timestamp_with_time_zone(Issues::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_issues_project_id")
                            .from(Issues::Table, Issues::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .index(
                        Index::create()
                            .unique()
                            .name("uidx_issues_project_id_number")
                            .col(Issues::ProjectId)
                            .col(Issues::Number),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Issues::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Issues {
    Table,
    Id,
    Token,
    ProjectId,
    Number,
    Title,
    Description,
    Status,
    Priority,
    Size,
    Slug,
    Version,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
}
