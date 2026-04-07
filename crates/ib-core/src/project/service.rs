use std::sync::Arc;

use super::model::{NewProject, NewProjectMember, Project, ProjectId, ProjectMember, ProjectToken};
use crate::{Error, RepositoryError, repository::RepositoryService, user::UserId, with_read_only_transaction, with_transaction};

#[async_trait::async_trait]
pub trait ProjectService: Send + Sync {
    async fn create_project(&self, new_project: NewProject) -> Result<Project, Error>;
    async fn update_project(&self, project: Project) -> Result<Project, Error>;
    async fn delete_project(&self, id: ProjectId) -> Result<Project, Error>;
    async fn find_by_id(&self, id: ProjectId) -> Result<Option<Project>, Error>;
    /// Look up a project by its opaque token. Resolves the token to its
    /// embedded ID — the token type ensures only tokens produced by this
    /// system are accepted.
    async fn find_by_token(&self, token: ProjectToken) -> Result<Option<Project>, Error>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Project>, Error>;
    async fn list_projects(&self) -> Result<Vec<Project>, Error>;
    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Project>, Error>;
    async fn add_member(&self, new_member: NewProjectMember) -> Result<ProjectMember, Error>;
    async fn update_member(&self, member: ProjectMember) -> Result<ProjectMember, Error>;
    async fn remove_member(&self, project_id: ProjectId, user_id: UserId) -> Result<(), Error>;
    async fn list_members(&self, project_id: ProjectId) -> Result<Vec<ProjectMember>, Error>;
    async fn find_member(&self, project_id: ProjectId, user_id: UserId) -> Result<Option<ProjectMember>, Error>;
}

pub(crate) struct ProjectServiceImpl {
    repository_service: Arc<RepositoryService>,
}

impl ProjectServiceImpl {
    pub(crate) fn new(repository_service: Arc<RepositoryService>) -> Self {
        Self { repository_service }
    }
}

#[async_trait::async_trait]
impl ProjectService for ProjectServiceImpl {
    async fn create_project(&self, new_project: NewProject) -> Result<Project, Error> {
        with_transaction!(self, project_repository, |tx| project_repository.create(tx, new_project).await)
    }

    async fn update_project(&self, project: Project) -> Result<Project, Error> {
        with_transaction!(self, project_repository, |tx| project_repository.update(tx, project).await)
    }

    async fn delete_project(&self, id: ProjectId) -> Result<Project, Error> {
        with_transaction!(self, project_repository, |tx| {
            let project = project_repository
                .find_by_id(tx, id)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            project_repository.delete(tx, project).await
        })
    }

    async fn find_by_id(&self, id: ProjectId) -> Result<Option<Project>, Error> {
        with_read_only_transaction!(self, project_repository, |tx| project_repository.find_by_id(tx, id).await)
    }

    async fn find_by_token(&self, token: ProjectToken) -> Result<Option<Project>, Error> {
        with_read_only_transaction!(self, project_repository, |tx| project_repository.find_by_id(tx, token.id()).await)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Project>, Error> {
        let slug = slug.to_owned();
        with_read_only_transaction!(self, project_repository, |tx| project_repository.find_by_slug(tx, &slug).await)
    }

    async fn list_projects(&self) -> Result<Vec<Project>, Error> {
        with_read_only_transaction!(self, project_repository, |tx| project_repository.list(tx).await)
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Project>, Error> {
        with_read_only_transaction!(self, project_repository, |tx| project_repository.list_for_user(tx, user_id).await)
    }

    async fn add_member(&self, new_member: NewProjectMember) -> Result<ProjectMember, Error> {
        with_transaction!(self, project_repository, project_member_repository, |tx| {
            project_repository
                .find_by_id(tx, new_member.project_id)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            project_member_repository.create(tx, new_member).await
        })
    }

    async fn update_member(&self, member: ProjectMember) -> Result<ProjectMember, Error> {
        with_transaction!(self, project_member_repository, |tx| project_member_repository.update(tx, member).await)
    }

    async fn remove_member(&self, project_id: ProjectId, user_id: UserId) -> Result<(), Error> {
        with_transaction!(self, project_member_repository, |tx| {
            project_member_repository
                .find(tx, project_id, user_id)
                .await?
                .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
            project_member_repository.delete(tx, project_id, user_id).await
        })
    }

    async fn list_members(&self, project_id: ProjectId) -> Result<Vec<ProjectMember>, Error> {
        with_read_only_transaction!(self, project_member_repository, |tx| project_member_repository.list(tx, project_id).await)
    }

    async fn find_member(&self, project_id: ProjectId, user_id: UserId) -> Result<Option<ProjectMember>, Error> {
        with_read_only_transaction!(self, project_member_repository, |tx| {
            project_member_repository.find(tx, project_id, user_id).await
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;

    use super::{ProjectService, ProjectServiceImpl};
    use crate::{
        Error, RepositoryError,
        project::{
            NewProject, NewProjectMember, Project, ProjectMember, ProjectToken,
            model::ProjectId,
            repository::{MockProjectMemberRepository, MockProjectRepository},
        },
        repository::testing::default_repository_service_builder,
        user::{Capabilities, UserId},
    };

    fn make_svc(project_repo: MockProjectRepository, member_repo: MockProjectMemberRepository) -> ProjectServiceImpl {
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .project_repository(Arc::new(project_repo))
                .project_member_repository(Arc::new(member_repo))
                .build()
                .unwrap(),
        );
        ProjectServiceImpl::new(repo_svc)
    }

    fn fake_project(id: ProjectId, slug: &str) -> Project {
        let now = Utc::now();
        Project {
            id,
            token: ProjectToken::new(id),
            name: format!("Project {slug}"),
            slug: slug.to_owned(),
            prefix: "TP".to_owned(),
            issue_counter: 0,
            description: None,
            version: 0,
            created_at: now,
            updated_at: now,
        }
    }

    fn fake_member(project_id: ProjectId, user_id: UserId) -> ProjectMember {
        let now = Utc::now();
        ProjectMember {
            project_id,
            user_id,
            capabilities: Capabilities::default(),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_project_delegates_to_repository() {
        let expected = fake_project(1, "myapp");
        let mut repo = MockProjectRepository::new();
        repo.expect_create().returning(move |_, _| {
            let p = expected.clone();
            Box::pin(async move { Ok(p) })
        });
        let svc = make_svc(repo, MockProjectMemberRepository::new());
        let result = svc.create_project(NewProject::new("MyApp", "myapp", "MA", None::<String>).unwrap()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().slug, "myapp");
    }

    #[tokio::test]
    async fn delete_project_not_found() {
        let mut repo = MockProjectRepository::new();
        repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_svc(repo, MockProjectMemberRepository::new());
        let result = svc.delete_project(999).await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    #[tokio::test]
    async fn delete_project_success() {
        let project = fake_project(1, "myapp");
        let deleted = project.clone();
        let mut repo = MockProjectRepository::new();
        repo.expect_find_by_id().returning(move |_, _| {
            let p = project.clone();
            Box::pin(async move { Ok(Some(p)) })
        });
        repo.expect_delete().returning(move |_, _| {
            let d = deleted.clone();
            Box::pin(async move { Ok(d) })
        });
        let svc = make_svc(repo, MockProjectMemberRepository::new());
        assert!(svc.delete_project(1).await.is_ok());
    }

    #[tokio::test]
    async fn add_member_fails_if_project_not_found() {
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));
        let svc = make_svc(project_repo, MockProjectMemberRepository::new());
        let result = svc
            .add_member(NewProjectMember {
                project_id: 999,
                user_id: 1,
                capabilities: Capabilities::default(),
            })
            .await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    #[tokio::test]
    async fn add_member_success() {
        let project = fake_project(1, "myapp");
        let member = fake_member(1, 1);
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_id().returning(move |_, _| {
            let p = project.clone();
            Box::pin(async move { Ok(Some(p)) })
        });
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_create().returning(move |_, _| {
            let m = member.clone();
            Box::pin(async move { Ok(m) })
        });
        let svc = make_svc(project_repo, member_repo);
        let result = svc
            .add_member(NewProjectMember {
                project_id: 1,
                user_id: 1,
                capabilities: Capabilities::default(),
            })
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn remove_member_not_found() {
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        let svc = make_svc(MockProjectRepository::new(), member_repo);
        let result = svc.remove_member(1, 1).await;
        assert!(matches!(result, Err(Error::RepositoryError(RepositoryError::NotFound))));
    }

    #[tokio::test]
    async fn remove_member_success() {
        let member = fake_member(1, 1);
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(move |_, _, _| {
            let m = member.clone();
            Box::pin(async move { Ok(Some(m)) })
        });
        member_repo.expect_delete().returning(|_, _, _| Box::pin(async { Ok(()) }));
        let svc = make_svc(MockProjectRepository::new(), member_repo);
        assert!(svc.remove_member(1, 1).await.is_ok());
    }

    #[tokio::test]
    async fn list_for_user_delegates_to_repository() {
        let projects = vec![fake_project(1, "myapp"), fake_project(2, "otherapp")];
        let expected = projects.clone();
        let mut repo = MockProjectRepository::new();
        repo.expect_list_for_user().returning(move |_, _| {
            let p = expected.clone();
            Box::pin(async move { Ok(p) })
        });
        let svc = make_svc(repo, MockProjectMemberRepository::new());
        let result = svc.list_for_user(42).await.unwrap();
        assert_eq!(result.len(), 2);
    }
}
