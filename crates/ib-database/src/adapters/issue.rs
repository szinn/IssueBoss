use ib_core::{
    Error, RepositoryError,
    issue::{Issue, IssueFilter, IssueId, IssueToken, NewIssueRecord, repository::IssueRepository},
    project::ProjectId,
    repository::Transaction,
};
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set};

use crate::{
    entities::{issues, prelude},
    handle_dberr,
    transaction::TransactionImpl,
};

pub(crate) struct IssueRepositoryAdapter;

impl IssueRepositoryAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl From<issues::Model> for Issue {
    fn from(model: issues::Model) -> Self {
        let token = IssueToken::new(model.id as u64);
        Self {
            id: model.id as u64,
            token,
            number: model.number as u32,
            project_id: model.project_id as u64,
            title: model.title,
            description: model.description,
            status: model.status.parse().unwrap_or(ib_core::issue::IssueStatus::Triage),
            priority: model.priority.parse().unwrap_or_default(),
            size: model.size.and_then(|s| s.parse().ok()),
            slug: model.slug,
            version: model.version as u64,
            created_at: model.created_at.with_timezone(&chrono::Utc),
            updated_at: model.updated_at.with_timezone(&chrono::Utc),
        }
    }
}

#[async_trait::async_trait]
impl IssueRepository for IssueRepositoryAdapter {
    async fn create(&self, transaction: &dyn Transaction, record: NewIssueRecord) -> Result<Issue, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let model = issues::ActiveModel {
            project_id: Set(record.project_id as i64),
            number: Set(record.number as i32),
            title: Set(record.title),
            description: Set(record.description),
            status: Set(record.status.to_string()),
            priority: Set(record.priority.to_string()),
            size: Set(record.size.map(|s| s.to_string())),
            slug: Set(record.slug),
            ..issues::ActiveModel::new()
        };
        let inserted = model.insert(db).await.map_err(handle_dberr)?;
        Ok(inserted.into())
    }

    async fn find_by_id(&self, transaction: &dyn Transaction, id: IssueId) -> Result<Option<Issue>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::Issues::find_by_id(id as i64).one(db).await.map_err(handle_dberr)?.map(Into::into))
    }

    async fn update(&self, transaction: &dyn Transaction, issue: Issue) -> Result<Issue, Error> {
        if issue.id == 0 {
            return Err(Error::InvalidId(issue.id));
        }
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let existing = prelude::Issues::find_by_id(issue.id as i64)
            .one(db)
            .await
            .map_err(handle_dberr)?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;

        if existing.version != issue.version as i64 {
            return Err(Error::RepositoryError(RepositoryError::Conflict));
        }

        let mut updater: issues::ActiveModel = existing.into();
        updater.title = Set(issue.title);
        updater.description = Set(issue.description);
        updater.status = Set(issue.status.to_string());
        updater.priority = Set(issue.priority.to_string());
        updater.size = Set(issue.size.map(|s| s.to_string()));
        let result = updater.update(db).await.map_err(handle_dberr)?;
        Ok(result.into())
    }

    async fn list(&self, transaction: &dyn Transaction, project_id: ProjectId, filter: IssueFilter) -> Result<Vec<Issue>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let mut query = prelude::Issues::find().filter(issues::Column::ProjectId.eq(project_id as i64));

        if let Some(status) = filter.status {
            query = query.filter(issues::Column::Status.eq(status.to_string()));
        }
        if let Some(priority) = filter.priority {
            query = query.filter(issues::Column::Priority.eq(priority.to_string()));
        }
        if let Some(size) = filter.size {
            query = query.filter(issues::Column::Size.eq(size.to_string()));
        }
        if let Some(limit) = filter.limit {
            query = query.limit(limit);
        }

        let mut rows: Vec<issues::Model> = query.all(db).await.map_err(handle_dberr)?;

        // Sort by priority rank (Urgent=0, High=1, Medium=2, Low=3), then number ASC
        rows.sort_by(|a, b| {
            let rank = |p: &str| match p {
                "Urgent" => 0u8,
                "High" => 1,
                "Medium" => 2,
                "Low" => 3,
                _ => 4,
            };
            rank(&a.priority).cmp(&rank(&b.priority)).then(a.number.cmp(&b.number))
        });

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{
        Error, RepositoryError,
        issue::{IssueFilter, IssuePriority, IssueStatus, NewIssueRecord},
        project::NewProject,
        repository::RepositoryService,
    };
    use sea_orm::Database;

    use crate::create_repository_service;

    async fn setup() -> Arc<RepositoryService> {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        create_repository_service(db).await.unwrap()
    }

    #[tokio::test]
    async fn create_and_find_by_id() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("myapp", "myapp", "MA", None::<String>).unwrap())
            .await
            .unwrap();
        let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let record = NewIssueRecord {
            project_id: project.id,
            number,
            title: "Fix login".into(),
            description: "desc".into(),
            status: IssueStatus::Triage,
            priority: IssuePriority::High,
            size: None,
            slug: format!("MA-{number}-fix-login"),
        };
        let created = svc.issue_repository().create(&*tx, record).await.unwrap();
        let found = svc.issue_repository().find_by_id(&*tx, created.id).await.unwrap().unwrap();
        assert_eq!(found.title, "Fix login");
        assert_eq!(found.number, 1);
        assert_eq!(found.project_id, project.id);
    }

    #[tokio::test]
    async fn list_returns_issues_for_project_sorted_by_priority() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("proj", "proj", "PR", None::<String>).unwrap())
            .await
            .unwrap();

        for (title, priority) in [("First", IssuePriority::Low), ("Second", IssuePriority::Urgent)] {
            let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
            let record = NewIssueRecord {
                project_id: project.id,
                number,
                title: title.into(),
                description: "".into(),
                status: IssueStatus::Triage,
                priority,
                size: None,
                slug: format!("PR-{number}-{}", title.to_lowercase()),
            };
            svc.issue_repository().create(&*tx, record).await.unwrap();
        }

        let issues = svc.issue_repository().list(&*tx, project.id, IssueFilter::default()).await.unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(issues[0].title, "Second"); // Urgent sorts first
        assert_eq!(issues[1].title, "First");
    }

    #[tokio::test]
    async fn update_increments_version() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("upd", "upd", "UP", None::<String>).unwrap())
            .await
            .unwrap();
        let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let record = NewIssueRecord {
            project_id: project.id,
            number,
            title: "Old title".into(),
            description: "".into(),
            status: IssueStatus::Triage,
            priority: IssuePriority::Medium,
            size: None,
            slug: "UP-1-old-title".into(),
        };
        let created = svc.issue_repository().create(&*tx, record).await.unwrap();
        let v0 = created.version;
        let mut to_update = created;
        to_update.title = "New title".into();
        let updated = svc.issue_repository().update(&*tx, to_update).await.unwrap();
        assert_eq!(updated.title, "New title");
        assert_eq!(updated.version, v0 + 1);
    }

    #[tokio::test]
    async fn update_conflict_on_stale_version() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("cfl", "cfl", "CF", None::<String>).unwrap())
            .await
            .unwrap();
        let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let record = NewIssueRecord {
            project_id: project.id,
            number,
            title: "Issue".into(),
            description: "".into(),
            status: IssueStatus::Triage,
            priority: IssuePriority::Medium,
            size: None,
            slug: "CF-1-issue".into(),
        };
        let mut issue = svc.issue_repository().create(&*tx, record).await.unwrap();
        issue.version = 99;
        let err = svc.issue_repository().update(&*tx, issue).await.unwrap_err();
        assert!(matches!(err, Error::RepositoryError(RepositoryError::Conflict)));
    }

    #[tokio::test]
    async fn increment_issue_counter_increments_sequentially() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("cnt", "cnt", "CN", None::<String>).unwrap())
            .await
            .unwrap();
        let n1 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let n2 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let n3 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        assert_eq!(n1, 1);
        assert_eq!(n2, 2);
        assert_eq!(n3, 3);
    }

    #[tokio::test]
    async fn list_filter_by_status() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("flt", "flt", "FL", None::<String>).unwrap())
            .await
            .unwrap();

        for status in [IssueStatus::Triage, IssueStatus::InDev, IssueStatus::Done] {
            let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
            let record = NewIssueRecord {
                project_id: project.id,
                number,
                title: format!("{status}"),
                description: "".into(),
                status: status.clone(),
                priority: IssuePriority::Medium,
                size: None,
                slug: format!("FL-{number}-{}", status.to_string().to_lowercase()),
            };
            svc.issue_repository().create(&*tx, record).await.unwrap();
        }

        let filter = IssueFilter {
            status: Some(IssueStatus::InDev),
            ..Default::default()
        };
        let issues = svc.issue_repository().list(&*tx, project.id, filter).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].status, IssueStatus::InDev);
    }
}
