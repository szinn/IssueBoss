use chrono::Utc;
use rand::RngExt;
use sea_orm::{ActiveValue::Set, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "api_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    pub user_id: i64,
    pub name: Option<String>,
    pub key_type: String,
    #[sea_orm(unique)]
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: DateTimeWithTimeZone,
    pub last_used_at: Option<DateTimeWithTimeZone>,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let id = rand::rng().random_range(1i64..=i64::MAX);
        Self {
            id: Set(id),
            created_at: Set(Utc::now().into()),
            ..ActiveModelTrait::default()
        }
    }
}
