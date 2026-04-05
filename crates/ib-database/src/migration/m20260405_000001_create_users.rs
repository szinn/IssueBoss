use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).big_integer().not_null().primary_key())
                    .col(ColumnDef::new(Users::Token).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::Username).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::FullName).string().not_null())
                    .col(ColumnDef::new(Users::PasswordHash).string().not_null())
                    .col(ColumnDef::new(Users::EmailAddress).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::ApiKeyHash).string().unique_key())
                    .col(ColumnDef::new(Users::ApiKeyPrefix).string())
                    .col(ColumnDef::new(Users::ApiKeyCreatedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Users::ApiKeyLastUsedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Users::Capabilities).json().not_null().default("[]"))
                    .col(ColumnDef::new(Users::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Users::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Token,
    Username,
    FullName,
    PasswordHash,
    EmailAddress,
    ApiKeyHash,
    ApiKeyPrefix,
    ApiKeyCreatedAt,
    ApiKeyLastUsedAt,
    Capabilities,
    CreatedAt,
    UpdatedAt,
}
