use std::sync::Arc;

use ib_core::CoreServices;
use tonic::{Request, Response, Status};

use crate::grpc::{
    admin_proto::{
        AddProjectMemberRequest, CreateApiKeyRequest, CreateApiKeyResponse, CreateIssueRequest, CreateProjectRequest, CreateUserRequest, DeleteProjectRequest,
        DeleteUserRequest, Empty, GetIssueRequest, GetProjectRequest, GetUserRequest, IssueResponse, ListApiKeysRequest, ListApiKeysResponse,
        ListIssuesRequest, ListIssuesResponse, ListProjectMembersRequest, ListProjectMembersResponse, ListProjectsRequest, ListProjectsResponse,
        ListUsersRequest, ListUsersResponse, ProjectMemberResponse, ProjectResponse, RemoveProjectMemberRequest, RevokeApiKeyRequest, SuperAdminRequest,
        SuperAdminResponse, TransitionIssueRequest, UpdateIssueRequest, UpdateProjectMemberRequest, UpdateProjectRequest, UpdateUserRequest, UserResponse,
        admin_service_server::AdminService,
    },
    error::map_core_error,
};

pub mod api_key;
pub mod issue;
pub mod project;
pub mod super_admin;
pub mod user;

fn user_to_proto(user: ib_core::user::User) -> UserResponse {
    UserResponse {
        token: user.token.to_string(),
        username: user.username,
        full_name: user.full_name,
        email: user.email_address,
        capabilities: user.capabilities.0.iter().map(|c| format!("{c:?}")).collect(),
        change_password_on_login: user.change_password_on_login,
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
    }
}

fn project_to_proto(project: ib_core::project::Project) -> ProjectResponse {
    ProjectResponse {
        token: project.token.to_string(),
        name: project.name,
        slug: project.slug,
        prefix: project.prefix,
        description: project.description,
        created_at: project.created_at.to_rfc3339(),
        updated_at: project.updated_at.to_rfc3339(),
    }
}

fn issue_to_proto(issue: ib_core::issue::Issue, project_slug: &str) -> IssueResponse {
    IssueResponse {
        token: issue.token.to_string(),
        project_slug: project_slug.to_owned(),
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

fn project_member_to_proto(member: ib_core::project::ProjectMember, user: &ib_core::user::User) -> ProjectMemberResponse {
    ProjectMemberResponse {
        project_token: ib_core::project::ProjectToken::new(member.project_id).to_string(),
        user_token: user.token.to_string(),
        username: user.username.clone(),
        capabilities: member.capabilities.0.iter().map(|c| format!("{c:?}")).collect(),
        created_at: member.created_at.to_rfc3339(),
        updated_at: member.updated_at.to_rfc3339(),
    }
}

pub struct GrpcAdminService {
    core_services: Arc<CoreServices>,
}

impl GrpcAdminService {
    pub(crate) fn new(core_services: Arc<CoreServices>) -> Self {
        Self { core_services }
    }
}

#[tonic::async_trait]
impl AdminService for GrpcAdminService {
    async fn super_admin(&self, request: Request<SuperAdminRequest>) -> Result<Response<SuperAdminResponse>, Status> {
        let response = super_admin::handler::super_admin(&self.core_services, request.into_inner())
            .await
            .map_err(map_core_error)?;
        Ok(Response::new(response))
    }

    async fn create_user(&self, request: Request<CreateUserRequest>) -> Result<Response<UserResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::create_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn get_user(&self, request: Request<GetUserRequest>) -> Result<Response<UserResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::get_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn list_users(&self, request: Request<ListUsersRequest>) -> Result<Response<ListUsersResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::list_users(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn update_user(&self, request: Request<UpdateUserRequest>) -> Result<Response<UserResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::update_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn delete_user(&self, request: Request<DeleteUserRequest>) -> Result<Response<Empty>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        user::handler::delete_user(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn create_api_key(&self, request: Request<CreateApiKeyRequest>) -> Result<Response<CreateApiKeyResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        api_key::handler::create_api_key(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn revoke_api_key(&self, request: Request<RevokeApiKeyRequest>) -> Result<Response<Empty>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        api_key::handler::revoke_api_key(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn list_api_keys(&self, request: Request<ListApiKeysRequest>) -> Result<Response<ListApiKeysResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        api_key::handler::list_api_keys(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn create_project(&self, request: Request<CreateProjectRequest>) -> Result<Response<ProjectResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::create_project(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn update_project(&self, request: Request<UpdateProjectRequest>) -> Result<Response<ProjectResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::update_project(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn delete_project(&self, request: Request<DeleteProjectRequest>) -> Result<Response<Empty>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::delete_project(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn get_project(&self, request: Request<GetProjectRequest>) -> Result<Response<ProjectResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::get_project(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn list_projects(&self, request: Request<ListProjectsRequest>) -> Result<Response<ListProjectsResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::list_projects(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn add_project_member(&self, request: Request<AddProjectMemberRequest>) -> Result<Response<ProjectMemberResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::add_project_member(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn update_project_member(&self, request: Request<UpdateProjectMemberRequest>) -> Result<Response<ProjectMemberResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::update_project_member(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn remove_project_member(&self, request: Request<RemoveProjectMemberRequest>) -> Result<Response<Empty>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::remove_project_member(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn list_project_members(&self, request: Request<ListProjectMembersRequest>) -> Result<Response<ListProjectMembersResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        project::handler::list_project_members(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn create_issue(&self, request: Request<CreateIssueRequest>) -> Result<Response<IssueResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        issue::handler::create_issue(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn update_issue(&self, request: Request<UpdateIssueRequest>) -> Result<Response<IssueResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        issue::handler::update_issue(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn get_issue(&self, request: Request<GetIssueRequest>) -> Result<Response<IssueResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        issue::handler::get_issue(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn list_issues(&self, request: Request<ListIssuesRequest>) -> Result<Response<ListIssuesResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        issue::handler::list_issues(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }

    async fn transition_issue(&self, request: Request<TransitionIssueRequest>) -> Result<Response<IssueResponse>, Status> {
        let user = crate::auth::authenticate_grpc(&self.core_services, request.metadata()).await?;
        crate::auth::require_admin(&user)?;
        issue::handler::transition_issue(&self.core_services, request.into_inner())
            .await
            .map(Response::new)
            .map_err(map_core_error)
    }
}

pub mod api {
    use ib_core::Error;

    use crate::{error::ApiError, grpc::admin_proto::admin_service_client::AdminServiceClient};

    pub(crate) async fn make_client(host: &str, port: u16) -> Result<AdminServiceClient<tonic::transport::Channel>, Error> {
        let uri = format!("{}:{port}", host.trim_end_matches('/'));
        let endpoint = tonic::transport::Channel::from_shared(uri.clone()).map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?;
        let endpoint = if uri.starts_with("https://") {
            let tls = tonic::transport::ClientTlsConfig::new().with_native_roots();
            endpoint.tls_config(tls).map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?
        } else {
            endpoint
        };
        Ok(AdminServiceClient::new(
            endpoint.connect().await.map_err(|e| Error::from(ApiError::GrpcClient(e.to_string())))?,
        ))
    }

    pub(crate) fn with_api_key<T>(mut req: tonic::Request<T>) -> tonic::Request<T> {
        if let Ok(key) = std::env::var("ISSUEBOSS_API_KEY") {
            if let Ok(val) = key.parse() {
                req.metadata_mut().insert("x-api-key", val);
            }
        }
        req
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ib_core::{
        CoreServices,
        api_key::{ApiKey, MockApiKeyRepository},
        create_services,
        user::MockUserRepository,
    };

    use super::GrpcAdminService;

    pub(crate) fn make_core_services(user_repo: MockUserRepository, api_key_repo: MockApiKeyRepository) -> Arc<CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(user_repo))
                .api_key_repository(Arc::new(api_key_repo))
                .build()
                .unwrap(),
        );
        create_services(repo_svc)
    }

    pub(crate) fn make_service_with_repos(user_repo: MockUserRepository, api_key_repo: MockApiKeyRepository) -> GrpcAdminService {
        GrpcAdminService::new(make_core_services(user_repo, api_key_repo))
    }

    pub(crate) fn fake_user(id: u64, username: &str) -> ib_core::user::User {
        use chrono::Utc;
        ib_core::user::User {
            id,
            token: ib_core::user::UserToken::new(id),
            username: username.to_owned(),
            full_name: format!("Test {username}"),
            password_hash: "hash".to_owned(),
            email_address: format!("{username}@example.com"),
            capabilities: ib_core::user::Capabilities::default(),
            change_password_on_login: false,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub(crate) fn fake_api_key(id: u64, user_id: u64) -> ApiKey {
        use chrono::Utc;
        ApiKey {
            id,
            user_id,
            key_type: "ib_live".to_owned(),
            key_hash: "hash".to_owned(),
            key_prefix: "ib_live_XXXX".to_owned(),
            name: None,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }
}
