use ib_core::{
    Error, RepositoryError,
    issue::{Issue, IssueFilter, IssueId, IssueToken, NewIssueRecord, repository::IssueRepository},
    project::ProjectId,
    repository::Transaction,
};
use sea_orm::{ActiveModelBehavior, ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, sea_query::Expr};

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
            status: model.status.parse().unwrap_or(ib_core::issue::IssueStatus::TriageNeeded),
            priority: model.priority.parse().unwrap_or_default(),
            size: model.size.and_then(|s| s.parse().ok()),
            slug: model.slug,
            version: model.version as u64,
            submitter: model.submitter_id as u64,
            assigned: model.assigned_id.map(|id| id as u64),
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
            submitter_id: Set(record.submitted_by as i64),
            assigned_id: Set(None),
            ..issues::ActiveModel::new()
        };
        let inserted = model.insert(db).await.map_err(handle_dberr)?;
        Ok(inserted.into())
    }

    async fn find_by_id(&self, transaction: &dyn Transaction, id: IssueId) -> Result<Option<Issue>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::Issues::find_by_id(id as i64).one(db).await.map_err(handle_dberr)?.map(Into::into))
    }

    async fn find_by_slug(&self, transaction: &dyn Transaction, slug: &str) -> Result<Option<Issue>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::Issues::find()
            .filter(issues::Column::Slug.eq(slug))
            .one(db)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
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
        updater.assigned_id = Set(issue.assigned.map(|id| id as i64));

        let result = updater.update(db).await.map_err(handle_dberr)?;

        Ok(result.into())
    }

    async fn list(&self, transaction: &dyn Transaction, project_id: ProjectId, filter: IssueFilter) -> Result<Vec<Issue>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let mut query = prelude::Issues::find().filter(issues::Column::ProjectId.eq(project_id as i64));

        if let Some(status) = filter.status {
            query = query.filter(issues::Column::Status.eq(status.to_string()));
        }
        if !filter.status_not_in.is_empty() {
            let excluded: Vec<String> = filter.status_not_in.iter().map(|s| s.to_string()).collect();
            query = query.filter(issues::Column::Status.is_not_in(excluded));
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
        if filter.exclude_blocked == Some(true) {
            query = query.filter(Expr::cust(
                "NOT EXISTS (SELECT 1 FROM issue_relationships ir INNER JOIN issues blocker ON blocker.id = ir.to_issue_id WHERE ir.from_issue_id = issues.id \
                 AND ir.kind = 'DependsOn' AND blocker.status != 'Done')",
            ));
        }
        if let Some(submitted_by) = filter.submitted_by {
            query = query.filter(issues::Column::SubmitterId.eq(submitted_by as i64));
        }
        if let Some(assigned_to) = filter.assigned_to {
            query = query.filter(issues::Column::AssignedId.eq(assigned_to as i64));
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
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::High,
            size: None,
            slug: format!("MA-{number}"),
            submitted_by: 1,
        };
        let created = svc.issue_repository().create(&*tx, record).await.unwrap();
        let found = svc.issue_repository().find_by_id(&*tx, created.id).await.unwrap().unwrap();
        assert_eq!(found.title, "Fix login");
        assert_eq!(found.number, 1);
        assert_eq!(found.project_id, project.id);
    }

    #[tokio::test]
    async fn find_by_slug_returns_issue() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("slugtest", "slugtest", "ST", None::<String>).unwrap())
            .await
            .unwrap();
        let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let record = NewIssueRecord {
            project_id: project.id,
            number,
            title: "Slug test".into(),
            description: "".into(),
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("ST-{number}"),
            submitted_by: 1,
        };
        svc.issue_repository().create(&*tx, record).await.unwrap();
        let found = svc.issue_repository().find_by_slug(&*tx, "ST-1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().slug, "ST-1");
        let missing = svc.issue_repository().find_by_slug(&*tx, "ST-999").await.unwrap();
        assert!(missing.is_none());
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
                status: IssueStatus::TriageNeeded,
                priority,
                size: None,
                slug: format!("PR-{number}"),
                submitted_by: 1,
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
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: "UP-1".into(),
            submitted_by: 1,
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
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: "CF-1".into(),
            submitted_by: 1,
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

        for status in [IssueStatus::TriageNeeded, IssueStatus::DevInProgress, IssueStatus::Done] {
            let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
            let record = NewIssueRecord {
                project_id: project.id,
                number,
                title: format!("{status}"),
                description: "".into(),
                status,
                priority: IssuePriority::Medium,
                size: None,
                slug: format!("FL-{number}"),
                submitted_by: 1,
            };
            svc.issue_repository().create(&*tx, record).await.unwrap();
        }

        let filter = IssueFilter {
            status: Some(IssueStatus::DevInProgress),
            ..Default::default()
        };
        let issues = svc.issue_repository().list(&*tx, project.id, filter).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].status, IssueStatus::DevInProgress);
    }

    #[tokio::test]
    async fn list_filter_by_status_not_in() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("opn", "opn", "OP", None::<String>).unwrap())
            .await
            .unwrap();

        for status in [IssueStatus::DevInProgress, IssueStatus::Done, IssueStatus::Backlog] {
            let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
            let record = NewIssueRecord {
                project_id: project.id,
                number,
                title: format!("{status}"),
                description: "".into(),
                status,
                priority: IssuePriority::Medium,
                size: None,
                slug: format!("OP-{number}"),
                submitted_by: 1,
            };
            svc.issue_repository().create(&*tx, record).await.unwrap();
        }

        let filter = IssueFilter {
            status_not_in: vec![IssueStatus::Done, IssueStatus::Canceled, IssueStatus::Backlog],
            ..Default::default()
        };
        let issues = svc.issue_repository().list(&*tx, project.id, filter).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert!(issues.iter().all(|i| i.status.is_open()));
        assert_eq!(issues[0].status, IssueStatus::DevInProgress);
        assert!(!issues.iter().any(|i| matches!(i.status, IssueStatus::Done | IssueStatus::Backlog)));
    }

    #[tokio::test]
    async fn exclude_blocked_filter() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();

        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("blk", "blk", "BK", None::<String>).unwrap())
            .await
            .unwrap();

        let n1 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let blocker = svc
            .issue_repository()
            .create(
                &*tx,
                NewIssueRecord {
                    project_id: project.id,
                    number: n1,
                    title: "Blocker".into(),
                    description: "".into(),
                    status: IssueStatus::DevInProgress,
                    priority: IssuePriority::Medium,
                    size: None,
                    slug: format!("BK-{n1}"),
                    submitted_by: 1,
                },
            )
            .await
            .unwrap();

        let n2 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let blocked = svc
            .issue_repository()
            .create(
                &*tx,
                NewIssueRecord {
                    project_id: project.id,
                    number: n2,
                    title: "Blocked".into(),
                    description: "".into(),
                    status: IssueStatus::DevNeeded,
                    priority: IssuePriority::Medium,
                    size: None,
                    slug: format!("BK-{n2}"),
                    submitted_by: 1,
                },
            )
            .await
            .unwrap();

        let n3 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let free = svc
            .issue_repository()
            .create(
                &*tx,
                NewIssueRecord {
                    project_id: project.id,
                    number: n3,
                    title: "Free".into(),
                    description: "".into(),
                    status: IssueStatus::DevNeeded,
                    priority: IssuePriority::Medium,
                    size: None,
                    slug: format!("BK-{n3}"),
                    submitted_by: 1,
                },
            )
            .await
            .unwrap();

        // blocked DependsOn blocker
        svc.relationship_repository()
            .add(
                &*tx,
                ib_core::relationship::model::NewIssueRelationship {
                    from_issue_id: blocked.id,
                    to_issue_id: blocker.id,
                    kind: ib_core::relationship::model::RelationshipKind::DependsOn,
                },
            )
            .await
            .unwrap();

        // Without filter: all three returned
        let all = svc.issue_repository().list(&*tx, project.id, IssueFilter::default()).await.unwrap();
        assert_eq!(all.len(), 3);

        // With exclude_blocked: only blocker and free returned (blocked is omitted)
        let filter = IssueFilter {
            exclude_blocked: Some(true),
            ..Default::default()
        };
        let unblocked = svc.issue_repository().list(&*tx, project.id, filter).await.unwrap();
        assert_eq!(unblocked.len(), 2);
        let slugs: Vec<&str> = unblocked.iter().map(|i| i.slug.as_str()).collect();
        assert!(slugs.contains(&blocker.slug.as_str()));
        assert!(slugs.contains(&free.slug.as_str()));
        assert!(!slugs.contains(&blocked.slug.as_str()));
    }

    #[tokio::test]
    async fn exclude_blocked_done_unblocks() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();

        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("res", "res", "RS", None::<String>).unwrap())
            .await
            .unwrap();

        let n1 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let done_blocker = svc
            .issue_repository()
            .create(
                &*tx,
                NewIssueRecord {
                    project_id: project.id,
                    number: n1,
                    title: "Done Blocker".into(),
                    description: "".into(),
                    status: IssueStatus::Done,
                    priority: IssuePriority::Medium,
                    size: None,
                    slug: format!("RS-{n1}"),
                    submitted_by: 1,
                },
            )
            .await
            .unwrap();

        let n2 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let issue = svc
            .issue_repository()
            .create(
                &*tx,
                NewIssueRecord {
                    project_id: project.id,
                    number: n2,
                    title: "Depends on Done".into(),
                    description: "".into(),
                    status: IssueStatus::DevNeeded,
                    priority: IssuePriority::Medium,
                    size: None,
                    slug: format!("RS-{n2}"),
                    submitted_by: 1,
                },
            )
            .await
            .unwrap();

        // issue DependsOn done_blocker — Done unblocks, so issue should appear
        svc.relationship_repository()
            .add(
                &*tx,
                ib_core::relationship::model::NewIssueRelationship {
                    from_issue_id: issue.id,
                    to_issue_id: done_blocker.id,
                    kind: ib_core::relationship::model::RelationshipKind::DependsOn,
                },
            )
            .await
            .unwrap();

        let filter = IssueFilter {
            exclude_blocked: Some(true),
            ..Default::default()
        };
        let result = svc.issue_repository().list(&*tx, project.id, filter).await.unwrap();

        // Both issues returned — Done blocker doesn't count as an active block
        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn exclude_blocked_canceled_still_blocks() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();

        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("can", "can", "CN", None::<String>).unwrap())
            .await
            .unwrap();

        let n1 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let canceled_blocker = svc
            .issue_repository()
            .create(
                &*tx,
                NewIssueRecord {
                    project_id: project.id,
                    number: n1,
                    title: "Canceled Blocker".into(),
                    description: "".into(),
                    status: IssueStatus::Canceled,
                    priority: IssuePriority::Medium,
                    size: None,
                    slug: format!("CN-{n1}"),
                    submitted_by: 1,
                },
            )
            .await
            .unwrap();

        let n2 = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let issue = svc
            .issue_repository()
            .create(
                &*tx,
                NewIssueRecord {
                    project_id: project.id,
                    number: n2,
                    title: "Depends on Canceled".into(),
                    description: "".into(),
                    status: IssueStatus::DevNeeded,
                    priority: IssuePriority::Medium,
                    size: None,
                    slug: format!("CN-{n2}"),
                    submitted_by: 1,
                },
            )
            .await
            .unwrap();

        // issue DependsOn canceled_blocker — Canceled does NOT unblock
        svc.relationship_repository()
            .add(
                &*tx,
                ib_core::relationship::model::NewIssueRelationship {
                    from_issue_id: issue.id,
                    to_issue_id: canceled_blocker.id,
                    kind: ib_core::relationship::model::RelationshipKind::DependsOn,
                },
            )
            .await
            .unwrap();

        let filter = IssueFilter {
            exclude_blocked: Some(true),
            ..Default::default()
        };
        let result = svc.issue_repository().list(&*tx, project.id, filter).await.unwrap();

        // canceled_blocker has no dependencies of its own, so it passes the filter.
        // issue is excluded because it depends on canceled_blocker (Canceled != Done,
        // so still an active block).
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].slug, canceled_blocker.slug);
    }

    #[tokio::test]
    async fn create_sets_submitter_and_assigned_is_null() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("subtest", "subtest", "SB", None::<String>).unwrap())
            .await
            .unwrap();
        let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let record = NewIssueRecord {
            project_id: project.id,
            number,
            title: "Submitter test".into(),
            description: "".into(),
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("SB-{number}"),
            submitted_by: 42,
        };
        let created = svc.issue_repository().create(&*tx, record).await.unwrap();
        assert_eq!(created.submitter, 42);
        assert_eq!(created.assigned, None);
    }

    #[tokio::test]
    async fn update_sets_assigned_id() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("assigntest", "assigntest", "AT", None::<String>).unwrap())
            .await
            .unwrap();
        let number = svc.project_repository().increment_issue_counter(&*tx, project.id).await.unwrap();
        let record = NewIssueRecord {
            project_id: project.id,
            number,
            title: "Assign test".into(),
            description: "".into(),
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("AT-{number}"),
            submitted_by: 1,
        };
        let mut issue = svc.issue_repository().create(&*tx, record).await.unwrap();
        issue.assigned = Some(7);
        let updated = svc.issue_repository().update(&*tx, issue).await.unwrap();
        assert_eq!(updated.assigned, Some(7));
        assert_eq!(updated.submitter, 1); // submitter is immutable
    }
}
