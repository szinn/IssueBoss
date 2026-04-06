use chrono::Utc;
use ib_core::{
    Error, RepositoryError,
    project::{NewProject, Project, ProjectId, ProjectToken, repository::ProjectRepository},
    repository::Transaction,
    user::UserId,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, QueryFilter, Set};

use crate::{
    entities::{
        prelude, project_members,
        projects::{self, Entity as ProjectsEntity},
    },
    handle_dberr,
    transaction::TransactionImpl,
};

pub(crate) struct ProjectRepositoryAdapter;

impl ProjectRepositoryAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl From<projects::Model> for Project {
    fn from(model: projects::Model) -> Self {
        Self {
            id: model.id as u64,
            token: ProjectToken::new(model.id as u64),
            name: model.name,
            slug: model.slug,
            prefix: model.prefix,
            issue_counter: model.issue_counter as u32,
            created_at: model.created_at.with_timezone(&chrono::Utc),
            updated_at: model.updated_at.with_timezone(&chrono::Utc),
        }
    }
}

#[async_trait::async_trait]
impl ProjectRepository for ProjectRepositoryAdapter {
    async fn list(&self, transaction: &dyn Transaction) -> Result<Vec<Project>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(ProjectsEntity::find()
            .all(db)
            .await
            .map_err(handle_dberr)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn list_for_user(&self, transaction: &dyn Transaction, user_id: UserId) -> Result<Vec<Project>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let member_rows = prelude::ProjectMembers::find()
            .filter(project_members::Column::UserId.eq(user_id as i64))
            .all(db)
            .await
            .map_err(handle_dberr)?;
        let project_ids: Vec<i64> = member_rows.iter().map(|m| m.project_id).collect();
        if project_ids.is_empty() {
            return Ok(vec![]);
        }
        Ok(ProjectsEntity::find()
            .filter(projects::Column::Id.is_in(project_ids))
            .all(db)
            .await
            .map_err(handle_dberr)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn find_by_id(&self, transaction: &dyn Transaction, id: ProjectId) -> Result<Option<Project>, Error> {
        if id == 0 {
            return Err(Error::InvalidId(id));
        }
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::Projects::find_by_id(id as i64).one(db).await.map_err(handle_dberr)?.map(Into::into))
    }

    async fn find_by_slug(&self, transaction: &dyn Transaction, slug: &str) -> Result<Option<Project>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(ProjectsEntity::find()
            .filter(projects::Column::Slug.eq(slug))
            .one(db)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn create(&self, transaction: &dyn Transaction, new_project: NewProject) -> Result<Project, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let token = ProjectToken::generate();
        let now = Utc::now();
        let model = projects::ActiveModel {
            id: Set(token.id() as i64),
            token: Set(token.to_string()),
            name: Set(new_project.name),
            slug: Set(new_project.slug),
            prefix: Set(new_project.prefix),
            issue_counter: Set(0),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };
        let model = model.insert(db).await.map_err(handle_dberr)?;
        Ok(model.into())
    }

    async fn update(&self, transaction: &dyn Transaction, project: Project) -> Result<Project, Error> {
        if project.id == 0 {
            return Err(Error::InvalidId(project.id));
        }
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let existing = prelude::Projects::find_by_id(project.id as i64)
            .one(db)
            .await
            .map_err(handle_dberr)?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let mut updater: projects::ActiveModel = existing.into();
        updater.name = Set(project.name);
        let result = updater.update(db).await.map_err(handle_dberr)?;
        Ok(result.into())
    }

    async fn delete(&self, transaction: &dyn Transaction, project: Project) -> Result<Project, Error> {
        if project.id == 0 {
            return Err(Error::InvalidId(project.id));
        }
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let existing = prelude::Projects::find_by_id(project.id as i64)
            .one(db)
            .await
            .map_err(handle_dberr)?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        // No optimistic concurrency guard: Project has no version field.
        let result: Project = existing.clone().into();
        existing.delete(db).await.map_err(handle_dberr)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{project::NewProject, repository::RepositoryService};
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
        let created = svc
            .project_repository()
            .create(&*tx, NewProject::new("MyApp", "myapp", "MA").unwrap())
            .await
            .unwrap();
        let found = svc.project_repository().find_by_id(&*tx, created.id).await.unwrap().unwrap();
        assert_eq!(found.slug, "myapp");
        assert_eq!(found.prefix, "MA");
    }

    #[tokio::test]
    async fn create_and_find_by_slug() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        svc.project_repository()
            .create(&*tx, NewProject::new("MyApp", "myapp", "MA").unwrap())
            .await
            .unwrap();
        let found = svc.project_repository().find_by_slug(&*tx, "myapp").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "MyApp");
    }

    #[tokio::test]
    async fn list_for_user_returns_only_member_projects() {
        use ib_core::{
            project::NewProjectMember,
            user::{Capabilities, NewUser},
        };
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let user = svc
            .user_repository()
            .create(
                &*tx,
                NewUser::new("alice", "pass", "alice@example.com", "Alice", Capabilities::default(), false).unwrap(),
            )
            .await
            .unwrap();
        let p1 = svc
            .project_repository()
            .create(&*tx, NewProject::new("App1", "app1", "APP").unwrap())
            .await
            .unwrap();
        svc.project_repository()
            .create(&*tx, NewProject::new("App2", "app2", "APT").unwrap())
            .await
            .unwrap();
        svc.project_member_repository()
            .create(
                &*tx,
                NewProjectMember {
                    project_id: p1.id,
                    user_id: user.id,
                    capabilities: Capabilities::default(),
                },
            )
            .await
            .unwrap();
        let projects = svc.project_repository().list_for_user(&*tx, user.id).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].slug, "app1");
    }

    #[tokio::test]
    async fn delete_project_succeeds() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let p = svc
            .project_repository()
            .create(&*tx, NewProject::new("Del", "del", "DL").unwrap())
            .await
            .unwrap();
        svc.project_repository().delete(&*tx, p.clone()).await.unwrap();
        let found = svc.project_repository().find_by_id(&*tx, p.id).await.unwrap();
        assert!(found.is_none());
    }
}
