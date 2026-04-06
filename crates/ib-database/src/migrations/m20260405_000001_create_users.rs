use sea_orm_migration::{
    prelude::*,
    schema::{big_integer, boolean, string, timestamp_with_time_zone, timestamp_with_time_zone_null},
};

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
                    .col(big_integer(Users::Id).not_null().primary_key())
                    .col(string(Users::Token).not_null().unique_key())
                    .col(string(Users::Username).not_null().unique_key())
                    .col(string(Users::FullName).not_null())
                    .col(string(Users::PasswordHash).not_null())
                    .col(string(Users::EmailAddress).not_null().unique_key())
                    .col(string(Users::ApiKeyHash).null().unique_key())
                    .col(string(Users::ApiKeyPrefix).null())
                    .col(timestamp_with_time_zone_null(Users::ApiKeyCreatedAt))
                    .col(timestamp_with_time_zone_null(Users::ApiKeyLastUsedAt))
                    .col(string(Users::Capabilities))
                    .col(boolean(Users::ChangePasswordOnLogin).not_null().default(false))
                    .col(big_integer(Users::Version))
                    .col(timestamp_with_time_zone(Users::CreatedAt))
                    .col(timestamp_with_time_zone(Users::UpdatedAt))
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
    ChangePasswordOnLogin,
    Version,
    CreatedAt,
    UpdatedAt,
}
