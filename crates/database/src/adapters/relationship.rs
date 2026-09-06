use std::str::FromStr;

use ib_core::{
    Error,
    issue::IssueId,
    relationship::{
        IssueRelationshipRepository,
        model::{IssueRelationship, IssueRelationships, NewIssueRelationship, RelatedIssueSummary, RelationshipKind},
    },
    repository::Transaction,
};
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, Set, Statement};

use crate::{entities::issue_relationships, handle_dberr, transaction::TransactionImpl};

fn list_for_issue_sql(backend: sea_orm::DatabaseBackend, id: i64) -> (String, Vec<sea_orm::Value>) {
    let params: Vec<sea_orm::Value> = vec![id.into(), id.into(), id.into(), id.into()];
    let sql = match backend {
        sea_orm::DatabaseBackend::Postgres => {
            "SELECT i.id, i.slug, i.title, 'from' AS direction FROM issue_relationships r JOIN issues i ON i.id = r.to_issue_id WHERE r.from_issue_id = $1 AND \
             r.kind = 'DependsOn' UNION ALL SELECT i.id, i.slug, i.title, 'to' AS direction FROM issue_relationships r JOIN issues i ON i.id = r.from_issue_id \
             WHERE r.to_issue_id = $2 AND r.kind = 'DependsOn' UNION ALL SELECT i.id, i.slug, i.title, 'related' AS direction FROM issue_relationships r JOIN \
             issues i ON i.id = r.to_issue_id WHERE r.from_issue_id = $3 AND r.kind = 'RelatedTo' UNION ALL SELECT i.id, i.slug, i.title, 'related' AS \
             direction FROM issue_relationships r JOIN issues i ON i.id = r.from_issue_id WHERE r.to_issue_id = $4 AND r.kind = 'RelatedTo'"
        }
        _ => {
            "SELECT i.id, i.slug, i.title, 'from' AS direction FROM issue_relationships r JOIN issues i ON i.id = r.to_issue_id WHERE r.from_issue_id = ? AND \
             r.kind = 'DependsOn' UNION ALL SELECT i.id, i.slug, i.title, 'to' AS direction FROM issue_relationships r JOIN issues i ON i.id = r.from_issue_id \
             WHERE r.to_issue_id = ? AND r.kind = 'DependsOn' UNION ALL SELECT i.id, i.slug, i.title, 'related' AS direction FROM issue_relationships r JOIN \
             issues i ON i.id = r.to_issue_id WHERE r.from_issue_id = ? AND r.kind = 'RelatedTo' UNION ALL SELECT i.id, i.slug, i.title, 'related' AS \
             direction FROM issue_relationships r JOIN issues i ON i.id = r.from_issue_id WHERE r.to_issue_id = ? AND r.kind = 'RelatedTo'"
        }
    };
    (sql.to_owned(), params)
}

impl From<issue_relationships::Model> for IssueRelationship {
    fn from(m: issue_relationships::Model) -> Self {
        IssueRelationship {
            id: m.id,
            from_issue_id: m.from_issue_id as u64,
            to_issue_id: m.to_issue_id as u64,
            kind: RelationshipKind::from_str(&m.kind).expect("valid kind in DB"),
            created_at: m.created_at.with_timezone(&chrono::Utc),
        }
    }
}

pub(crate) struct IssueRelationshipRepositoryAdapter;

impl IssueRelationshipRepositoryAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[derive(Debug, FromQueryResult)]
struct RelatedIssueSummaryRow {
    id: i64,
    slug: String,
    title: String,
    direction: String,
}

#[async_trait::async_trait]
impl IssueRelationshipRepository for IssueRelationshipRepositoryAdapter {
    async fn add(&self, transaction: &dyn Transaction, record: NewIssueRelationship) -> Result<IssueRelationship, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let model = issue_relationships::ActiveModel {
            from_issue_id: Set(record.from_issue_id as i64),
            to_issue_id: Set(record.to_issue_id as i64),
            kind: Set(record.kind.to_string()),
            ..issue_relationships::ActiveModel::new()
        };
        let inserted = model.insert(db).await.map_err(handle_dberr)?;
        Ok(inserted.into())
    }

    async fn remove(&self, transaction: &dyn Transaction, from_issue_id: IssueId, to_issue_id: IssueId, kind: RelationshipKind) -> Result<bool, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;

        let result = issue_relationships::Entity::delete_many()
            .filter(issue_relationships::Column::FromIssueId.eq(from_issue_id as i64))
            .filter(issue_relationships::Column::ToIssueId.eq(to_issue_id as i64))
            .filter(issue_relationships::Column::Kind.eq(kind.to_string()))
            .exec(db)
            .await
            .map_err(handle_dberr)?;

        if result.rows_affected > 0 {
            return Ok(true);
        }

        // For RelatedTo, try reverse direction.
        if kind == RelationshipKind::RelatedTo {
            let result2 = issue_relationships::Entity::delete_many()
                .filter(issue_relationships::Column::FromIssueId.eq(to_issue_id as i64))
                .filter(issue_relationships::Column::ToIssueId.eq(from_issue_id as i64))
                .filter(issue_relationships::Column::Kind.eq(kind.to_string()))
                .exec(db)
                .await
                .map_err(handle_dberr)?;
            return Ok(result2.rows_affected > 0);
        }

        Ok(false)
    }

    async fn list_for_issue(&self, transaction: &dyn Transaction, issue_id: IssueId) -> Result<IssueRelationships, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let id = issue_id as i64;
        let backend = db.get_database_backend();

        // Kind values must match RelationshipKind::Display ("DependsOn",
        // "RelatedTo").
        let (sql, values) = list_for_issue_sql(backend, id);
        let stmt = Statement::from_sql_and_values(backend, sql, values);
        let rows = RelatedIssueSummaryRow::find_by_statement(stmt).all(db).await.map_err(handle_dberr)?;

        let mut result = IssueRelationships::default();
        for row in rows {
            let summary = RelatedIssueSummary {
                id: row.id as u64,
                slug: row.slug,
                title: row.title,
            };
            match row.direction.as_str() {
                "from" => result.depends_on.push(summary),
                "to" => result.blocks.push(summary),
                "related" => result.related_to.push(summary),
                _ => {}
            }
        }
        Ok(result)
    }
}
