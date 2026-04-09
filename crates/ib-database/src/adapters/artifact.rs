use ib_core::{
    Error,
    artifact::{
        ArtifactRepository,
        model::{ArtifactId, ArtifactKind, ArtifactToken, IssueArtifact, NewArtifact},
    },
    issue::IssueId,
    repository::Transaction,
};
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use crate::{
    entities::{issue_artifacts, prelude},
    handle_dberr,
    transaction::TransactionImpl,
};

pub(crate) struct ArtifactRepositoryAdapter;

impl ArtifactRepositoryAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ArtifactRepository for ArtifactRepositoryAdapter {
    async fn create(&self, transaction: &dyn Transaction, record: NewArtifact) -> Result<IssueArtifact, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let model = issue_artifacts::ActiveModel {
            issue_id: Set(record.issue_id as i64),
            kind: Set(record.kind.to_string()),
            slug: Set(record.slug),
            body: Set(record.body.to_string()),
            created_by: Set(record.created_by),
            ..issue_artifacts::ActiveModel::new()
        };
        let inserted = model.insert(db).await.map_err(handle_dberr)?;
        Ok(inserted.into())
    }

    async fn find_by_id(&self, transaction: &dyn Transaction, id: ArtifactId) -> Result<Option<IssueArtifact>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::IssueArtifacts::find_by_id(id as i64)
            .one(db)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn find_by_token(&self, transaction: &dyn Transaction, token: ArtifactToken) -> Result<Option<IssueArtifact>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::IssueArtifacts::find()
            .filter(issue_artifacts::Column::Token.eq(token.to_string()))
            .one(db)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn find_by_slug(&self, transaction: &dyn Transaction, issue_id: IssueId, slug: &str) -> Result<Option<IssueArtifact>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::IssueArtifacts::find()
            .filter(issue_artifacts::Column::IssueId.eq(issue_id as i64))
            .filter(issue_artifacts::Column::Slug.eq(slug))
            .one(db)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn list(&self, transaction: &dyn Transaction, issue_id: IssueId, kinds: Option<Vec<ArtifactKind>>) -> Result<Vec<IssueArtifact>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let mut query = prelude::IssueArtifacts::find().filter(issue_artifacts::Column::IssueId.eq(issue_id as i64));
        if let Some(k) = kinds {
            let kind_strs: Vec<String> = k.iter().map(|k| k.to_string()).collect();
            query = query.filter(issue_artifacts::Column::Kind.is_in(kind_strs));
        }
        Ok(query.all(db).await.map_err(handle_dberr)?.into_iter().map(Into::into).collect())
    }

    async fn update(&self, transaction: &dyn Transaction, artifact: IssueArtifact) -> Result<IssueArtifact, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let model = issue_artifacts::ActiveModel {
            id: Set(artifact.id as i64),
            token: Set(artifact.token.to_string()),
            issue_id: Set(artifact.issue_id as i64),
            kind: Set(artifact.kind.to_string()),
            slug: Set(artifact.slug),
            body: Set(artifact.body.to_string()),
            created_by: Set(artifact.created_by),
            created_at: Set(artifact.created_at.into()),
            updated_at: Set(artifact.updated_at.into()),
        };
        let updated = model.update(db).await.map_err(handle_dberr)?;
        Ok(updated.into())
    }

    async fn delete(&self, transaction: &dyn Transaction, id: ArtifactId) -> Result<(), Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        prelude::IssueArtifacts::delete_by_id(id as i64).exec(db).await.map_err(handle_dberr)?;
        Ok(())
    }
}
