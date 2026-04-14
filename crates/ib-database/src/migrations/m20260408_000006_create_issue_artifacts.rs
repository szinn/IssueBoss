use sea_orm_migration::{
    prelude::*,
    schema::{big_integer, string, text, timestamp_with_time_zone},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(IssueArtifacts::Table)
                    .if_not_exists()
                    .col(big_integer(IssueArtifacts::Id).not_null().primary_key())
                    .col(string(IssueArtifacts::Token).not_null().unique_key())
                    .col(big_integer(IssueArtifacts::IssueId).not_null())
                    .col(string(IssueArtifacts::Kind).not_null())
                    .col(text(IssueArtifacts::Body).not_null())
                    .col(string(IssueArtifacts::CreatedBy).not_null())
                    .col(string(IssueArtifacts::Slug).null())
                    .col(timestamp_with_time_zone(IssueArtifacts::CreatedAt))
                    .col(timestamp_with_time_zone(IssueArtifacts::UpdatedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_issue_artifacts_issue_id")
                            .from(IssueArtifacts::Table, IssueArtifacts::IssueId)
                            .to(Issues::Table, Issues::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_issue_artifacts_issue_id")
                    .table(IssueArtifacts::Table)
                    .col(IssueArtifacts::IssueId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_issue_artifacts_issue_id_slug")
                    .table(IssueArtifacts::Table)
                    .col(IssueArtifacts::IssueId)
                    .col(IssueArtifacts::Slug)
                    .unique()
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
enum IssueArtifacts {
    Table,
    Id,
    Token,
    IssueId,
    Kind,
    Body,
    Slug,
    CreatedBy,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Issues {
    Table,
    Id,
}
