use sea_orm_migration::{
    prelude::*,
    schema::{big_integer, string, timestamp_with_time_zone, timestamp_with_time_zone_null},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(big_integer(ApiKeys::Id).not_null().primary_key())
                    .col(big_integer(ApiKeys::UserId).not_null())
                    .col(string(ApiKeys::Name).null())
                    .col(string(ApiKeys::KeyType).not_null())
                    .col(string(ApiKeys::KeyHash).not_null().unique_key())
                    .col(string(ApiKeys::KeyPrefix).not_null())
                    .col(timestamp_with_time_zone(ApiKeys::CreatedAt))
                    .col(timestamp_with_time_zone_null(ApiKeys::LastUsedAt))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_api_keys_user_id")
                            .from(ApiKeys::Table, ApiKeys::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    UserId,
    KeyType,
    KeyHash,
    KeyPrefix,
    Name,
    CreatedAt,
    LastUsedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
