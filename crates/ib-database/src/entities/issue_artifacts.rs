use std::str::FromStr;

use chrono::Utc;
use ib_core::artifact::model::{ArtifactKind, ArtifactToken, IssueArtifact};
use sea_orm::{ActiveValue::Set, entity::prelude::*};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "issue_artifacts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    #[sea_orm(unique)]
    pub token: String,
    pub issue_id: i64,
    pub kind: String,
    pub body: String,
    pub created_by: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        let token = ArtifactToken::generate();
        let now = Utc::now();
        Self {
            id: Set(token.id() as i64),
            token: Set(token.to_string()),
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
            self.updated_at = Set(Utc::now().into());
        }
        Ok(self)
    }
}

impl From<Model> for IssueArtifact {
    fn from(m: Model) -> Self {
        IssueArtifact {
            id: m.id as u64,
            token: ArtifactToken::from_str(&m.token).expect("valid token in DB"),
            issue_id: m.issue_id as u64,
            kind: ArtifactKind::from_str(&m.kind).expect("valid kind in DB"),
            body: serde_json::from_str(&m.body).expect("valid JSON in DB"),
            created_by: m.created_by,
            created_at: m.created_at.with_timezone(&chrono::Utc),
            updated_at: m.updated_at.with_timezone(&chrono::Utc),
        }
    }
}
