use chrono::Utc;
use ib_core::issue::IssueToken;
use sea_orm::{ActiveValue::Set, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "issues")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    #[sea_orm(unique)]
    pub token: String,
    pub project_id: i64,
    pub number: i32,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub size: Option<String>,
    #[sea_orm(unique)]
    pub slug: String,
    pub version: i64,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let token = IssueToken::generate();
        let now = Utc::now();
        Self {
            id: Set(token.id() as i64),
            token: Set(token.to_string()),
            version: Set(0),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..ActiveModelTrait::default()
        }
    }

    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if self.is_changed() {
            self.version = Set(self.version.unwrap() + 1);
            self.updated_at = Set(Utc::now().into());
        }
        Ok(self)
    }
}
