use chrono::Utc;
use ib_core::{
    Error, RepositoryError,
    project::{NewProjectMember, ProjectId, ProjectMember, repository::ProjectMemberRepository},
    repository::Transaction,
    types::Capabilities,
    user::UserId,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, ModelTrait, QueryFilter, Set};

use crate::{
    entities::{
        prelude,
        project_members::{self, Entity as ProjectMembersEntity},
    },
    handle_dberr,
    transaction::TransactionImpl,
};

pub(crate) struct ProjectMemberRepositoryAdapter;

impl ProjectMemberRepositoryAdapter {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl From<project_members::Model> for ProjectMember {
    fn from(model: project_members::Model) -> Self {
        let capabilities: Capabilities = serde_json::from_str(&model.capabilities).unwrap_or_default();
        Self {
            project_id: model.project_id as u64,
            user_id: model.user_id as u64,
            capabilities,
            created_at: model.created_at.with_timezone(&chrono::Utc),
            updated_at: model.updated_at.with_timezone(&chrono::Utc),
        }
    }
}

#[async_trait::async_trait]
impl ProjectMemberRepository for ProjectMemberRepositoryAdapter {
    async fn list(&self, transaction: &dyn Transaction, project_id: ProjectId) -> Result<Vec<ProjectMember>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(ProjectMembersEntity::find()
            .filter(project_members::Column::ProjectId.eq(project_id as i64))
            .all(db)
            .await
            .map_err(handle_dberr)?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn find(&self, transaction: &dyn Transaction, project_id: ProjectId, user_id: UserId) -> Result<Option<ProjectMember>, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        Ok(prelude::ProjectMembers::find_by_id((project_id as i64, user_id as i64))
            .one(db)
            .await
            .map_err(handle_dberr)?
            .map(Into::into))
    }

    async fn create(&self, transaction: &dyn Transaction, new_member: NewProjectMember) -> Result<ProjectMember, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let caps = serde_json::to_string(&new_member.capabilities).map_err(|e| Error::Infrastructure(e.to_string()))?;
        let now = Utc::now();
        let model = project_members::ActiveModel {
            project_id: Set(new_member.project_id as i64),
            user_id: Set(new_member.user_id as i64),
            capabilities: Set(caps),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };
        let model = model.insert(db).await.map_err(handle_dberr)?;
        Ok(model.into())
    }

    async fn update(&self, transaction: &dyn Transaction, member: ProjectMember) -> Result<ProjectMember, Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let existing = prelude::ProjectMembers::find_by_id((member.project_id as i64, member.user_id as i64))
            .one(db)
            .await
            .map_err(handle_dberr)?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        let new_caps = serde_json::to_string(&member.capabilities).map_err(|e| Error::Infrastructure(e.to_string()))?;
        let mut updater: project_members::ActiveModel = existing.into();
        updater.capabilities = Set(new_caps);
        let result = updater.update(db).await.map_err(handle_dberr)?;
        Ok(result.into())
    }

    async fn delete(&self, transaction: &dyn Transaction, project_id: ProjectId, user_id: UserId) -> Result<(), Error> {
        let db = TransactionImpl::get_db_transaction(transaction)?;
        let existing = prelude::ProjectMembers::find_by_id((project_id as i64, user_id as i64))
            .one(db)
            .await
            .map_err(handle_dberr)?
            .ok_or(Error::RepositoryError(RepositoryError::NotFound))?;
        ModelTrait::delete(existing, db).await.map_err(handle_dberr)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{
        project::{NewProject, NewProjectMember},
        repository::RepositoryService,
        types::Capabilities,
        user::NewUser,
    };
    use sea_orm::Database;

    use crate::create_repository_service;

    async fn setup() -> Arc<RepositoryService> {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        create_repository_service(db).await.unwrap()
    }

    #[tokio::test]
    async fn create_and_find_member() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let user = svc
            .user_repository()
            .create(&*tx, NewUser::new("alice", "pass", "a@a.com", "Alice", Capabilities::default(), false).unwrap())
            .await
            .unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("P1", "p1", "PA", None::<String>).unwrap())
            .await
            .unwrap();
        svc.project_member_repository()
            .create(
                &*tx,
                NewProjectMember {
                    project_id: project.id,
                    user_id: user.id,
                    capabilities: Capabilities::default(),
                },
            )
            .await
            .unwrap();
        let found = svc.project_member_repository().find(&*tx, project.id, user.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn list_members_for_project() {
        let svc = setup().await;
        let tx = svc.repository().begin().await.unwrap();
        let u1 = svc
            .user_repository()
            .create(&*tx, NewUser::new("u1", "p", "u1@a.com", "U1", Capabilities::default(), false).unwrap())
            .await
            .unwrap();
        let u2 = svc
            .user_repository()
            .create(&*tx, NewUser::new("u2", "p", "u2@a.com", "U2", Capabilities::default(), false).unwrap())
            .await
            .unwrap();
        let project = svc
            .project_repository()
            .create(&*tx, NewProject::new("P2", "p2", "PB", None::<String>).unwrap())
            .await
            .unwrap();
        svc.project_member_repository()
            .create(
                &*tx,
                NewProjectMember {
                    project_id: project.id,
                    user_id: u1.id,
                    capabilities: Capabilities::default(),
                },
            )
            .await
            .unwrap();
        svc.project_member_repository()
            .create(
                &*tx,
                NewProjectMember {
                    project_id: project.id,
                    user_id: u2.id,
                    capabilities: Capabilities::default(),
                },
            )
            .await
            .unwrap();
        let members = svc.project_member_repository().list(&*tx, project.id).await.unwrap();
        assert_eq!(members.len(), 2);
    }
}
