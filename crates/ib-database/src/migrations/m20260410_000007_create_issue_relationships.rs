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
                    .table(IssueRelationships::Table)
                    .if_not_exists()
                    .col(big_integer(IssueRelationships::Id).not_null().primary_key().auto_increment())
                    .col(big_integer(IssueRelationships::FromIssueId).not_null())
                    .col(big_integer(IssueRelationships::ToIssueId).not_null())
                    .col(string(IssueRelationships::Kind).not_null())
                    .col(timestamp_with_time_zone(IssueRelationships::CreatedAt).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_issue_relationships_from")
                            .from(IssueRelationships::Table, IssueRelationships::FromIssueId)
                            .to(Issues::Table, Issues::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_issue_relationships_to")
                            .from(IssueRelationships::Table, IssueRelationships::ToIssueId)
                            .to(Issues::Table, Issues::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_issue_relationships_unique")
                    .table(IssueRelationships::Table)
                    .col(IssueRelationships::FromIssueId)
                    .col(IssueRelationships::ToIssueId)
                    .col(IssueRelationships::Kind)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_issue_relationships_from")
                    .table(IssueRelationships::Table)
                    .col(IssueRelationships::FromIssueId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_issue_relationships_to")
                    .table(IssueRelationships::Table)
                    .col(IssueRelationships::ToIssueId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum IssueRelationships {
    Table,
    Id,
    FromIssueId,
    ToIssueId,
    Kind,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Issues {
    Table,
    Id,
}
