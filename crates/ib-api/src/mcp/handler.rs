use std::sync::Arc;

use axum::{Extension, Json, extract::State, http::StatusCode};
use ib_core::CoreServices;
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use ib_core::{
        api_key::MockApiKeyRepository,
        project::{MockProjectRepository, Project, ProjectToken},
        user::{MockUserRepository, User, UserToken},
    };

    use super::ProjectSummary;
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
}
