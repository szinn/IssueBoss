use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    #[sea_orm(unique)]
    pub token: String,
    #[sea_orm(unique)]
    pub username: String,
    pub full_name: String,
    pub password_hash: String,
    #[sea_orm(unique)]
    pub email_address: String,
    #[sea_orm(unique)]
    pub api_key_hash: Option<String>,
    pub api_key_prefix: Option<String>,
    pub api_key_created_at: Option<DateTimeWithTimeZone>,
    pub api_key_last_used_at: Option<DateTimeWithTimeZone>,
    pub capabilities: Json,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
