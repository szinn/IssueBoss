use super::model::{Issue, IssueFilter, IssueId, NewIssueRecord};
use crate::{Error, project::ProjectId, repository::Transaction};

#[async_trait::async_trait]
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
pub trait IssueRepository: Send + Sync {
    async fn create(&self, transaction: &dyn Transaction, record: NewIssueRecord) -> Result<Issue, Error>;
    async fn find_by_id(&self, transaction: &dyn Transaction, id: IssueId) -> Result<Option<Issue>, Error>;
    async fn find_by_slug(&self, transaction: &dyn Transaction, slug: &str) -> Result<Option<Issue>, Error>;
    async fn update(&self, transaction: &dyn Transaction, issue: Issue) -> Result<Issue, Error>;
    async fn list(&self, transaction: &dyn Transaction, project_id: ProjectId, filter: IssueFilter) -> Result<Vec<Issue>, Error>;
}
