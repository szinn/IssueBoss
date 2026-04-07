use ib_core::{
    Error,
    issue::{Issue, IssueFilter, IssueId, NewIssueRecord, repository::IssueRepository},
    project::ProjectId,
    repository::Transaction,
};

pub(crate) struct IssueRepositoryAdapter;

impl IssueRepositoryAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl IssueRepository for IssueRepositoryAdapter {
    async fn create(&self, _transaction: &dyn Transaction, _record: NewIssueRecord) -> Result<Issue, Error> {
        unimplemented!("IssueRepositoryAdapter::create: will be implemented in Task 4")
    }

    async fn find_by_id(&self, _transaction: &dyn Transaction, _id: IssueId) -> Result<Option<Issue>, Error> {
        unimplemented!("IssueRepositoryAdapter::find_by_id: will be implemented in Task 4")
    }

    async fn update(&self, _transaction: &dyn Transaction, _issue: Issue) -> Result<Issue, Error> {
        unimplemented!("IssueRepositoryAdapter::update: will be implemented in Task 4")
    }

    async fn list(&self, _transaction: &dyn Transaction, _project_id: ProjectId, _filter: IssueFilter) -> Result<Vec<Issue>, Error> {
        unimplemented!("IssueRepositoryAdapter::list: will be implemented in Task 4")
    }
}
