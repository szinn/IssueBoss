use std::sync::Arc;

use super::model::{Issue, IssueFilter, IssueId, IssueStatus, IssueToken, NewIssue, NewIssueRecord, derive_issue_slug};
use crate::{Error, RepositoryError, repository::RepositoryService, with_read_only_transaction, with_transaction};

#[async_trait::async_trait]
pub trait IssueService: Send + Sync {
    async fn create_issue(&self, new_issue: NewIssue) -> Result<Issue, Error>;
    async fn update_issue(&self, issue: Issue) -> Result<Issue, Error>;
    async fn find_by_id(&self, id: IssueId) -> Result<Option<Issue>, Error>;
    async fn find_by_token(&self, token: IssueToken) -> Result<Option<Issue>, Error>;
    async fn list_issues(&self, project_id: crate::project::ProjectId, filter: IssueFilter) -> Result<Vec<Issue>, Error>;
}

pub(crate) struct IssueServiceImpl {
    repository_service: Arc<RepositoryService>,
}

impl IssueServiceImpl {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        Self { repository_service }
    }
}

#[async_trait::async_trait]
impl IssueService for IssueServiceImpl {
    async fn create_issue(&self, new_issue: NewIssue) -> Result<Issue, Error> {
        with_transaction!(self, project_repository, issue_repository, |tx| {
            // Verify project exists and atomically get next issue number.
            project_repository
                .find_by_id(tx, new_issue.project_id)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            let number = project_repository.increment_issue_counter(tx, new_issue.project_id).await?;
            let slug = derive_issue_slug(&new_issue.project_prefix, number, &new_issue.title);
            issue_repository
                .create(
                    tx,
                    NewIssueRecord {
                        project_id: new_issue.project_id,
                        number,
                        title: new_issue.title,
                        description: new_issue.description,
                        status: IssueStatus::Triage,
                        priority: new_issue.priority,
                        size: new_issue.size,
                        slug,
                    },
                )
                .await
        })
    }

    async fn update_issue(&self, issue: Issue) -> Result<Issue, Error> {
        with_transaction!(self, issue_repository, |tx| issue_repository.update(tx, issue).await)
    }

    async fn find_by_id(&self, id: IssueId) -> Result<Option<Issue>, Error> {
        with_read_only_transaction!(self, issue_repository, |tx| issue_repository.find_by_id(tx, id).await)
    }

    async fn find_by_token(&self, token: IssueToken) -> Result<Option<Issue>, Error> {
        with_read_only_transaction!(self, issue_repository, |tx| issue_repository.find_by_id(tx, token.id()).await)
    }

    async fn list_issues(&self, project_id: crate::project::ProjectId, filter: IssueFilter) -> Result<Vec<Issue>, Error> {
        with_read_only_transaction!(self, issue_repository, |tx| issue_repository.list(tx, project_id, filter).await)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;

    use super::{IssueService, IssueServiceImpl};
    use crate::{
        Error, RepositoryError,
        issue::{Issue, IssueFilter, IssueId, IssuePriority, IssueStatus, IssueToken, NewIssue, repository::MockIssueRepository},
        project::{ProjectId, ProjectToken, model::Project, repository::MockProjectRepository},
        repository::testing::default_repository_service_builder,
    };

    fn fake_project(id: ProjectId) -> Project {
        let now = Utc::now();
        Project {
            id,
            token: ProjectToken::new(id),
            name: "Test".into(),
            slug: "test".into(),
            prefix: "TP".into(),
            issue_counter: 0,
            description: None,
            version: 0,
            created_at: now,
            updated_at: now,
        }
    }

    fn fake_issue(id: IssueId, project_id: ProjectId, number: u32) -> Issue {
        let now = Utc::now();
        Issue {
            id,
            token: IssueToken::new(id),
            number,
            project_id,
            title: format!("Issue {number}"),
            description: "".into(),
            status: IssueStatus::Triage,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("TP-{number}-issue-{number}"),
            created_at: now,
            updated_at: now,
        }
    }

    fn make_svc(project_repo: MockProjectRepository, issue_repo: MockIssueRepository) -> IssueServiceImpl {
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        IssueServiceImpl::new(repo_svc)
    }

    #[tokio::test]
    async fn create_issue_success() {
        let project = fake_project(1);
        let created = fake_issue(100, 1, 1);
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_id().returning(move |_, _| {
            let p = project.clone();
            Box::pin(async move { Ok(Some(p)) })
        });
        project_repo.expect_increment_issue_counter().returning(|_, _| Box::pin(async { Ok(1u32) }));
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_create().returning(move |_, _| {
            let i = created.clone();
            Box::pin(async move { Ok(i) })
        });
        let svc = make_svc(project_repo, issue_repo);
        let result = svc
            .create_issue(NewIssue::new(1, "TP", "Fix login", "", IssuePriority::High, None).unwrap())
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().number, 1);
    }

    #[tokio::test]
    async fn create_issue_project_not_found() {
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_svc(project_repo, MockIssueRepository::new());
        let result = svc
            .create_issue(NewIssue::new(999, "TP", "Fix", "", IssuePriority::Medium, None).unwrap())
            .await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    #[tokio::test]
    async fn list_issues_delegates_to_repository() {
        let issues = vec![fake_issue(1, 1, 1), fake_issue(2, 1, 2)];
        let expected = issues.clone();
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_list().returning(move |_, _, _| {
            let i = expected.clone();
            Box::pin(async move { Ok(i) })
        });
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc.list_issues(1, IssueFilter::default()).await.unwrap();
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn update_issue_delegates_to_repository() {
        let issue = fake_issue(10, 1, 3);
        let updated = fake_issue(10, 1, 3);
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_update().returning(move |_, _| {
            let i = updated.clone();
            Box::pin(async move { Ok(i) })
        });
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc.update_issue(issue).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 10);
    }

    #[tokio::test]
    async fn find_by_id_delegates_to_repository() {
        let issue = fake_issue(42, 1, 5);
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_id().withf(|_, id| *id == 42).returning(move |_, _| {
            let i = issue.clone();
            Box::pin(async move { Ok(Some(i)) })
        });
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc.find_by_id(42).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 42);
    }

    #[tokio::test]
    async fn find_by_token_delegates_using_id() {
        let issue = fake_issue(42, 1, 5);
        let token = IssueToken::new(42);
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_id().withf(|_, id| *id == 42).returning(move |_, _| {
            let i = issue.clone();
            Box::pin(async move { Ok(Some(i)) })
        });
        let svc = make_svc(MockProjectRepository::new(), issue_repo);
        let result = svc.find_by_token(token).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, 42);
    }
}
