use super::model::{IssueRelationship, IssueRelationships, NewIssueRelationship, RelationshipKind};
use crate::{Error, issue::IssueId, repository::Transaction};

#[async_trait::async_trait]
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
pub trait IssueRelationshipRepository: Send + Sync {
    async fn add(&self, transaction: &dyn Transaction, record: NewIssueRelationship) -> Result<IssueRelationship, Error>;

    async fn remove(&self, transaction: &dyn Transaction, from_issue_id: IssueId, to_issue_id: IssueId, kind: RelationshipKind) -> Result<bool, Error>;

    async fn list_for_issue(&self, transaction: &dyn Transaction, issue_id: IssueId) -> Result<IssueRelationships, Error>;

    async fn exists(&self, transaction: &dyn Transaction, from_issue_id: IssueId, to_issue_id: IssueId, kind: &RelationshipKind) -> Result<bool, Error>;
}
