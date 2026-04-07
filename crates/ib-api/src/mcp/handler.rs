use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use ib_core::{
    CoreServices,
    issue::{IssueFilter, IssuePriority, IssueSize, NewIssue},
    project::ProjectToken,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthenticatedUser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub token: String,
    pub name: String,
    pub slug: String,
    pub prefix: String,
}

impl From<ib_core::project::Project> for ProjectSummary {
    fn from(p: ib_core::project::Project) -> Self {
        Self {
            token: p.token.to_string(),
            name: p.name,
            slug: p.slug,
            prefix: p.prefix,
        }
    }
}

/// Handler function — returns projects the authenticated user is a member of.
pub async fn list_projects_handler(
    State(core_services): State<Arc<CoreServices>>,
    Extension(AuthenticatedUser(user)): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<ProjectSummary>>, StatusCode> {
    let projects = core_services
        .project_service()
        .list_for_user(user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(projects.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSummary {
    pub token: String,
    pub project_token: String,
    pub number: u32,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub size: Option<String>,
    pub slug: String,
    pub created_at: String,
    pub updated_at: String,
}

impl IssueSummary {
    fn from_issue(issue: ib_core::issue::Issue, project: &ib_core::project::Project) -> Self {
        Self {
            token: issue.token.to_string(),
            project_token: project.token.to_string(),
            number: issue.number,
            title: issue.title,
            description: issue.description,
            status: issue.status.to_string(),
            priority: issue.priority.to_string(),
            size: issue.size.map(|s| s.to_string()),
            slug: issue.slug,
            created_at: issue.created_at.to_rfc3339(),
            updated_at: issue.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateIssueBody {
    pub project_token: String,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub size: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIssueBody {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub size: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TransitionIssueBody {
    pub new_status: String,
}

#[derive(Debug, Deserialize)]
pub struct ListIssuesQuery {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub size: Option<String>,
    pub limit: Option<u64>,
}

pub async fn create_issue_handler(
    State(core): State<Arc<CoreServices>>,
    Extension(AuthenticatedUser(_user)): Extension<AuthenticatedUser>,
    Json(body): Json<CreateIssueBody>,
) -> Result<Json<IssueSummary>, StatusCode> {
    let project_token = ProjectToken::parse(&body.project_token).map_err(|_| StatusCode::BAD_REQUEST)?;
    let project = core
        .project_service()
        .find_by_token(project_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let priority = body
        .priority
        .as_deref()
        .map(|s| s.parse::<IssuePriority>().map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()?
        .unwrap_or(IssuePriority::Medium);
    let size = body
        .size
        .as_deref()
        .map(|s| s.parse::<IssueSize>().map_err(|_| StatusCode::BAD_REQUEST))
        .transpose()?;
    let new_issue =
        NewIssue::new(project.id, &project.prefix, body.title, body.description.unwrap_or_default(), priority, size).map_err(|_| StatusCode::BAD_REQUEST)?;
    let issue = core
        .issue_service()
        .create_issue(new_issue)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(IssueSummary::from_issue(issue, &project)))
}

pub async fn get_issue_handler(
    State(core): State<Arc<CoreServices>>,
    Extension(AuthenticatedUser(_user)): Extension<AuthenticatedUser>,
    Path(slug): Path<String>,
) -> Result<Json<IssueSummary>, StatusCode> {
    let issue = core
        .issue_service()
        .find_by_slug(&slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let project = core
        .project_service()
        .find_by_id(issue.project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(IssueSummary::from_issue(issue, &project)))
}

pub async fn update_issue_handler(
    State(core): State<Arc<CoreServices>>,
    Extension(AuthenticatedUser(_user)): Extension<AuthenticatedUser>,
    Path(slug): Path<String>,
    Json(body): Json<UpdateIssueBody>,
) -> Result<Json<IssueSummary>, StatusCode> {
    let mut issue = core
        .issue_service()
        .find_by_slug(&slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let project = core
        .project_service()
        .find_by_id(issue.project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if let Some(title) = body.title {
        issue.title = title;
    }
    if let Some(description) = body.description {
        issue.description = description;
    }
    if let Some(priority) = body.priority {
        issue.priority = priority.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    if let Some(size) = body.size {
        issue.size = Some(size.parse().map_err(|_| StatusCode::BAD_REQUEST)?);
    }
    let updated = core.issue_service().update_issue(issue).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(IssueSummary::from_issue(updated, &project)))
}

pub async fn transition_issue_handler(
    State(core): State<Arc<CoreServices>>,
    Extension(AuthenticatedUser(_user)): Extension<AuthenticatedUser>,
    Path(slug): Path<String>,
    Json(body): Json<TransitionIssueBody>,
) -> Result<Json<IssueSummary>, StatusCode> {
    use ib_core::{Error, RepositoryError, issue::IssueStatus};

    let new_status: IssueStatus = body.new_status.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let issue = core
        .issue_service()
        .find_by_slug(&slug)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let project = core
        .project_service()
        .find_by_id(issue.project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let updated = core.issue_service().transition_issue(issue.token, new_status).await.map_err(|e| match e {
        Error::Validation(_) => StatusCode::BAD_REQUEST,
        Error::RepositoryError(RepositoryError::NotFound) => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    })?;
    Ok(Json(IssueSummary::from_issue(updated, &project)))
}

pub async fn list_issues_handler(
    State(core): State<Arc<CoreServices>>,
    Extension(AuthenticatedUser(_user)): Extension<AuthenticatedUser>,
    Path(project_token_str): Path<String>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<Json<Vec<IssueSummary>>, StatusCode> {
    let project_token = ProjectToken::parse(&project_token_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    let project = core
        .project_service()
        .find_by_token(project_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let filter = IssueFilter {
        status: query.status.map(|s| s.parse().map_err(|_| StatusCode::BAD_REQUEST)).transpose()?,
        priority: query.priority.map(|s| s.parse().map_err(|_| StatusCode::BAD_REQUEST)).transpose()?,
        size: query.size.map(|s| s.parse().map_err(|_| StatusCode::BAD_REQUEST)).transpose()?,
        limit: query.limit,
    };
    let issues = core
        .issue_service()
        .list_issues(project.id, filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(issues.into_iter().map(|i| IssueSummary::from_issue(i, &project)).collect()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use ib_core::{
        api_key::MockApiKeyRepository,
        issue::{IssuePriority, IssueStatus, IssueToken, MockIssueRepository},
        project::{MockProjectRepository, Project, ProjectToken},
        user::{MockUserRepository, User, UserToken},
    };

    use super::{IssueSummary, ProjectSummary};
    use crate::auth::AuthenticatedUser;

    fn fake_project(id: u64, slug: &str) -> Project {
        Project {
            id,
            token: ProjectToken::new(id),
            name: format!("Project {slug}"),
            slug: slug.to_owned(),
            prefix: "TP".to_owned(),
            issue_counter: 0,
            description: None,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn fake_user(id: u64) -> User {
        User {
            id,
            token: UserToken::new(id),
            username: "alice".to_owned(),
            full_name: "Alice".to_owned(),
            password_hash: "h".to_owned(),
            email_address: "alice@example.com".to_owned(),
            capabilities: ib_core::user::Capabilities::default(),
            change_password_on_login: false,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn fake_issue(id: u64, project_id: u64, number: u32) -> ib_core::issue::Issue {
        ib_core::issue::Issue {
            id,
            token: IssueToken::new(id),
            number,
            project_id,
            title: format!("Issue {number}"),
            description: String::new(),
            status: IssueStatus::Triage,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("TP-{number}"),
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_core(project_repo: MockProjectRepository) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(MockUserRepository::new()))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(project_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    fn make_core_with_issues(project_repo: MockProjectRepository, issue_repo: MockIssueRepository) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(MockUserRepository::new()))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    #[test]
    fn project_summary_from_project() {
        let p = fake_project(1, "myapp");
        let s: ProjectSummary = p.into();
        assert_eq!(s.slug, "myapp");
        assert_eq!(s.prefix, "TP");
        assert!(s.token.starts_with("P_"));
    }

    #[tokio::test]
    async fn list_projects_returns_member_projects() {
        let user = fake_user(42);
        let projects = vec![fake_project(1, "app1"), fake_project(2, "app2")];
        let mut project_repo = MockProjectRepository::new();
        let user_id = user.id;
        project_repo.expect_list_for_user().withf(move |_, id| *id == user_id).returning(move |_, _| {
            let p = projects.clone();
            Box::pin(async move { Ok(p) })
        });
        let core = make_core(project_repo);
        let result = core.project_service().list_for_user(user.id).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].slug, "app1");
    }

    #[tokio::test]
    async fn list_projects_empty_when_no_membership() {
        let user = fake_user(99);
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_list_for_user().returning(|_, _| Box::pin(async { Ok(vec![]) }));
        let core = make_core(project_repo);
        let result = core.project_service().list_for_user(user.id).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn handler_maps_projects_to_summaries() {
        let user = fake_user(1);
        let projects = vec![fake_project(10, "proj")];
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_list_for_user().returning(move |_, _| {
            let p = projects.clone();
            Box::pin(async move { Ok(p) })
        });
        let core = make_core(project_repo);

        // Call handler directly
        let result = super::list_projects_handler(axum::extract::State(core), axum::Extension(AuthenticatedUser(user)))
            .await
            .unwrap();
        assert_eq!(result.0.len(), 1);
        assert_eq!(result.0[0].slug, "proj");
    }

    #[test]
    fn issue_summary_from_issue_sets_all_fields() {
        let issue = fake_issue(7, 1, 3);
        let project = fake_project(1, "myapp");
        let summary = IssueSummary::from_issue(issue, &project);
        assert!(summary.token.starts_with("I_"));
        assert!(summary.project_token.starts_with("P_"));
        assert_eq!(summary.number, 3);
        assert_eq!(summary.title, "Issue 3");
        assert_eq!(summary.status, "Triage");
        assert_eq!(summary.priority, "Medium");
        assert!(summary.size.is_none());
        assert_eq!(summary.slug, "TP-3");
    }

    #[tokio::test]
    async fn transition_issue_handler_success() {
        let user = fake_user(1);
        let project = fake_project(1, "myapp");
        let issue = fake_issue(10, 1, 1); // status: Triage
        let transitioned = {
            let mut i = issue.clone();
            i.status = IssueStatus::SpecNeeded;
            i
        };
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_id().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let t = transitioned.clone();
            issue_repo
                .expect_update()
                .withf(|_, i| i.status == IssueStatus::SpecNeeded)
                .returning(move |_, _| {
                    let t = t.clone();
                    Box::pin(async move { Ok(t) })
                });
        }
        let core = make_core_with_issues(project_repo, issue_repo);
        use axum::extract::{Path, State};
        let result = super::transition_issue_handler(
            State(core),
            axum::Extension(AuthenticatedUser(user)),
            Path("TP-1".into()),
            axum::Json(super::TransitionIssueBody {
                new_status: "SpecNeeded".into(),
            }),
        )
        .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0.status, "SpecNeeded");
    }

    #[tokio::test]
    async fn transition_issue_handler_unknown_status_returns_bad_request() {
        let user = fake_user(1);
        let project_repo = MockProjectRepository::new();
        let issue_repo = MockIssueRepository::new();
        let core = make_core_with_issues(project_repo, issue_repo);
        use axum::extract::{Path, State};
        let result = super::transition_issue_handler(
            State(core),
            axum::Extension(AuthenticatedUser(user)),
            Path("TP-1".into()),
            axum::Json(super::TransitionIssueBody {
                new_status: "NotARealStatus".into(),
            }),
        )
        .await;
        assert!(matches!(result, Err(axum::http::StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn transition_issue_handler_not_found_returns_404() {
        let user = fake_user(1);
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        let core = make_core_with_issues(project_repo, issue_repo);
        use axum::extract::{Path, State};
        let result = super::transition_issue_handler(
            State(core),
            axum::Extension(AuthenticatedUser(user)),
            Path("TP-999".into()),
            axum::Json(super::TransitionIssueBody {
                new_status: "SpecNeeded".into(),
            }),
        )
        .await;
        assert!(matches!(result, Err(axum::http::StatusCode::NOT_FOUND)));
    }

    #[tokio::test]
    async fn list_issues_handler_returns_issues_for_project() {
        let user = fake_user(1);
        let project = fake_project(5, "myapp");
        let issue = fake_issue(10, 5, 1);
        let project_token_str = project.token.to_string();
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_id().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_list().returning(move |_, _, _| {
                let i = i.clone();
                Box::pin(async move { Ok(vec![i]) })
            });
        }
        let core = make_core_with_issues(project_repo, issue_repo);
        use axum::extract::{Path, Query, State};
        let query = super::ListIssuesQuery {
            status: None,
            priority: None,
            size: None,
            limit: None,
        };
        let result = super::list_issues_handler(State(core), axum::Extension(AuthenticatedUser(user)), Path(project_token_str), Query(query))
            .await
            .unwrap();
        assert_eq!(result.0.len(), 1);
        assert!(result.0[0].project_token.starts_with("P_"));
        assert_eq!(result.0[0].number, 1);
    }
}
