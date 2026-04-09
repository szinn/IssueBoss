use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(IssueArtifacts::Table)
                    .add_column(ColumnDef::new(IssueArtifacts::Slug).string().null())
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

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(Index::drop().name("idx_issue_artifacts_issue_id_slug").table(IssueArtifacts::Table).to_owned())
            .await?;

        manager
            .alter_table(Table::alter().table(IssueArtifacts::Table).drop_column(IssueArtifacts::Slug).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum IssueArtifacts {
    Table,
    IssueId,
    Slug,
}
