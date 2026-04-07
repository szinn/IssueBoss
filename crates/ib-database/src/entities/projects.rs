use chrono::Utc;
use ib_core::project::ProjectToken;
use sea_orm::{ActiveValue::Set, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    #[sea_orm(unique)]
    pub token: String,
    pub name: String,
    #[sea_orm(unique)]
    pub slug: String,
    #[sea_orm(unique)]
    pub prefix: String,
    pub issue_counter: i32,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let token = ProjectToken::generate();

        Self {
            id: Set(token.id() as i64),
            token: Set(token.to_string()),
            issue_counter: Set(0),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
            ..ActiveModelTrait::default()
        }
    }

    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if self.is_changed() {
            self.updated_at = Set(Utc::now().into());
        }
        Ok(self)
    }
}
