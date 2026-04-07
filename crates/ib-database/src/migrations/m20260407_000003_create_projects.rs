use sea_orm_migration::{
    prelude::*,
    schema::{big_integer, integer, string, timestamp_with_time_zone},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(big_integer(Projects::Id).not_null().primary_key())
                    .col(string(Projects::Token).not_null().unique_key())
                    .col(string(Projects::Name).not_null())
                    .col(string(Projects::Slug).not_null().unique_key())
                    .col(string(Projects::Prefix).not_null().unique_key())
                    .col(integer(Projects::IssueCounter).not_null().default(0))
                    .col(string(Projects::Description).null())
                    .col(big_integer(Projects::Version).not_null().default(0))
                    .col(timestamp_with_time_zone(Projects::CreatedAt))
                    .col(timestamp_with_time_zone(Projects::UpdatedAt))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(Projects::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    Id,
    Token,
    Name,
    Slug,
    Prefix,
    IssueCounter,
    Description,
    Version,
    CreatedAt,
    UpdatedAt,
}
