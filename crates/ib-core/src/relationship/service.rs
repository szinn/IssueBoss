use super::model::{IssueRelationship, IssueRelationships, RelationshipKind};
use crate::{Error, issue::IssueId};

#[async_trait::async_trait]
pub trait IssueRelationshipService: Send + Sync {
    async fn add_relationship(&self, from_slug: &str, to_slug: &str, kind: RelationshipKind) -> Result<IssueRelationship, Error>;

    async fn remove_relationship(&self, from_slug: &str, to_slug: &str, kind: RelationshipKind) -> Result<bool, Error>;

    async fn list_for_issue(&self, issue_id: IssueId) -> Result<IssueRelationships, Error>;
}

#[allow(dead_code)]
pub(crate) struct IssueRelationshipServiceImpl;
