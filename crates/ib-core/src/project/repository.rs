use super::model::{NewProject, NewProjectMember, Project, ProjectId, ProjectMember};
use crate::{Error, repository::Transaction, user::UserId};

#[async_trait::async_trait]
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
pub trait ProjectRepository: Send + Sync {
    async fn list(&self, transaction: &dyn Transaction) -> Result<Vec<Project>, Error>;
    async fn list_for_user(&self, transaction: &dyn Transaction, user_id: UserId) -> Result<Vec<Project>, Error>;
    async fn find_by_id(&self, transaction: &dyn Transaction, id: ProjectId) -> Result<Option<Project>, Error>;
    async fn find_by_slug(&self, transaction: &dyn Transaction, slug: &str) -> Result<Option<Project>, Error>;
    async fn create(&self, transaction: &dyn Transaction, new_project: NewProject) -> Result<Project, Error>;
    async fn update(&self, transaction: &dyn Transaction, project: Project) -> Result<Project, Error>;
    async fn delete(&self, transaction: &dyn Transaction, project: Project) -> Result<Project, Error>;
    async fn increment_issue_counter(&self, transaction: &dyn Transaction, id: ProjectId) -> Result<u32, Error>;
}

#[async_trait::async_trait]
#[cfg_attr(any(test, feature = "test-support"), mockall::automock)]
pub trait ProjectMemberRepository: Send + Sync {
    async fn list(&self, transaction: &dyn Transaction, project_id: ProjectId) -> Result<Vec<ProjectMember>, Error>;
    async fn find(&self, transaction: &dyn Transaction, project_id: ProjectId, user_id: UserId) -> Result<Option<ProjectMember>, Error>;
    async fn create(&self, transaction: &dyn Transaction, new_member: NewProjectMember) -> Result<ProjectMember, Error>;
    async fn update(&self, transaction: &dyn Transaction, member: ProjectMember) -> Result<ProjectMember, Error>;
    async fn delete(&self, transaction: &dyn Transaction, project_id: ProjectId, user_id: UserId) -> Result<(), Error>;
}
