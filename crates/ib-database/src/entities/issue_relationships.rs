use std::str::FromStr;

use chrono::Utc;
use ib_core::relationship::model::{IssueRelationship, RelationshipKind};
use sea_orm::{ActiveValue::Set, entity::prelude::*};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "issue_relationships")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,
    pub from_issue_id: i64,
    pub to_issue_id: i64,
    pub kind: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            created_at: Set(Utc::now().into()),
            ..ActiveModelTrait::default()
        }
    }
}

impl From<Model> for IssueRelationship {
    fn from(m: Model) -> Self {
        IssueRelationship {
            id: m.id,
            from_issue_id: m.from_issue_id as u64,
            to_issue_id: m.to_issue_id as u64,
            kind: RelationshipKind::from_str(&m.kind).expect("valid kind in DB"),
            created_at: m.created_at.with_timezone(&chrono::Utc),
        }
    }
}
