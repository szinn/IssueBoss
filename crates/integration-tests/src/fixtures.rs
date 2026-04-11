use ib_core::{
    CoreServices,
    api_key::{ApiKey, NewApiKey},
    issue::{Issue, IssuePriority, NewIssue},
    project::{NewProject, NewProjectMember, Project, ProjectId, ProjectMember},
    types::Capabilities,
    user::{NewUser, User, UserId},
};

pub async fn create_user(services: &CoreServices, username: &str) -> User {
    let new_user = NewUser::new(
        username,
        "password123",
        format!("{username}@example.com"),
        format!("Test {username}"),
        Capabilities::default(),
        false,
    )
    .expect("valid new user");
    services.user_service().create_user(new_user).await.expect("create_user fixture failed")
}

pub async fn create_project(services: &CoreServices, name: &str, slug: &str, prefix: &str) -> Project {
    let new_project = NewProject::new(name, slug, prefix, None::<String>).expect("valid new project");
    services
        .project_service()
        .create_project(new_project)
        .await
        .expect("create_project fixture failed")
}

pub async fn create_api_key(services: &CoreServices, user_id: UserId) -> ApiKey {
    let new_key = NewApiKey {
        user_id,
        key_type: "ib_test".to_owned(),
        name: None,
    };
    let (key, _plaintext) = services.api_key_service().create_key(new_key).await.expect("create_api_key fixture failed");
    key
}

pub async fn add_project_member(services: &CoreServices, project_id: ProjectId, user_id: UserId) -> ProjectMember {
    services
        .project_service()
        .add_member(NewProjectMember {
            project_id,
            user_id,
            capabilities: Capabilities::default(),
        })
        .await
        .expect("add_project_member fixture failed")
}

pub async fn create_issue(services: &CoreServices, project_id: ProjectId, prefix: &str, title: &str) -> Issue {
    let new_issue = NewIssue::new(project_id, prefix, title, "", IssuePriority::Medium, None, 0).expect("valid new issue");
    services.issue_service().create_issue(new_issue).await.expect("create_issue fixture failed")
}
