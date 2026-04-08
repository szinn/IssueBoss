use super::model::{ArtifactId, ArtifactKind, ArtifactToken, IssueArtifact, NewArtifact};
use crate::{Error, issue::IssueId, repository::Transaction};

#[async_trait::async_trait]
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
pub trait ArtifactRepository: Send + Sync {
    async fn create(&self, transaction: &dyn Transaction, record: NewArtifact) -> Result<IssueArtifact, Error>;
    async fn find_by_id(&self, transaction: &dyn Transaction, id: ArtifactId) -> Result<Option<IssueArtifact>, Error>;
    async fn find_by_token(&self, transaction: &dyn Transaction, token: ArtifactToken) -> Result<Option<IssueArtifact>, Error>;
    async fn list(&self, transaction: &dyn Transaction, issue_id: IssueId, kinds: Option<Vec<ArtifactKind>>) -> Result<Vec<IssueArtifact>, Error>;
    async fn update(&self, transaction: &dyn Transaction, artifact: IssueArtifact) -> Result<IssueArtifact, Error>;
    async fn delete(&self, transaction: &dyn Transaction, id: ArtifactId) -> Result<(), Error>;
}
