use std::{collections::HashMap, sync::Arc};

use ib_core::{
    CoreServices,
    artifact::{ArtifactKind, IssueArtifact, NewArtifact},
    issue::{IssueFilter, IssueId, IssuePriority, IssueSize, IssueStatus, NewIssue},
    project::{Project, ProjectId},
    types::Capability,
    user::{User, UserId, UserToken},
};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        Annotated, ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams, RawResource, RawResourceTemplate, ReadResourceRequestParams,
        ReadResourceResult, ResourceContents, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};

// ---------------------------------------------------------------------------
// Serialisable output structs
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct ProjectSummaryMcp {
    pub token: String,
    pub name: String,
    pub slug: String,
    pub prefix: String,
    pub description: Option<String>,
}

impl From<Project> for ProjectSummaryMcp {
    fn from(p: Project) -> Self {
        Self {
            token: p.token.to_string(),
            name: p.name,
            slug: p.slug,
            prefix: p.prefix,
            description: p.description,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct IssueSummaryMcp {
    pub token: String,
    pub number: u32,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub size: Option<String>,
    pub slug: String,
    pub submitter: UserRefMcp,
    pub assigned: Option<UserRefMcp>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct UserRefMcp {
    pub token: String,
    pub username: String,
    pub full_name: String,
}

#[derive(Debug, serde::Serialize)]
struct ArtifactMcp {
    token: String,
    slug: Option<String>,
    kind: String,
    body: serde_json::Value,
    created_by: String,
    created_at: String,
    updated_at: String,
}

impl ArtifactMcp {
    fn from_artifact(a: IssueArtifact) -> Self {
        Self {
            token: a.token.to_string(),
            slug: a.slug,
            kind: a.kind.to_string(),
            body: a.body,
            created_by: a.created_by,
            created_at: a.created_at.to_rfc3339(),
            updated_at: a.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct TransitionIssueMcp {
    issue: IssueSummaryMcp,
    completeness: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
struct MovedArtifactMcp {
    issue_slug: String,
    slug: Option<String>,
    kind: String,
}

#[derive(Debug, serde::Serialize)]
struct MoveArtifactResultMcp {
    updated: usize,
    artifacts: Vec<MovedArtifactMcp>,
}

#[derive(Debug, serde::Serialize)]
struct RelatedIssueSummaryMcp {
    id: u64,
    slug: String,
    title: String,
}

#[derive(Debug, serde::Serialize)]
struct IssueRelationshipsMcp {
    depends_on: Vec<RelatedIssueSummaryMcp>,
    blocks: Vec<RelatedIssueSummaryMcp>,
    related_to: Vec<RelatedIssueSummaryMcp>,
}

impl From<ib_core::relationship::IssueRelationships> for IssueRelationshipsMcp {
    fn from(r: ib_core::relationship::IssueRelationships) -> Self {
        let convert = |v: Vec<ib_core::relationship::RelatedIssueSummary>| {
            v.into_iter()
                .map(|s| RelatedIssueSummaryMcp {
                    id: s.id,
                    slug: s.slug,
                    title: s.title,
                })
                .collect()
        };
        Self {
            depends_on: convert(r.depends_on),
            blocks: convert(r.blocks),
            related_to: convert(r.related_to),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool parameter structs
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListIssuesParams {
    /// Project slug, e.g. "issueboss"
    pub project_slug: String,
    /// Filter by status, e.g. "TriageNeeded"
    pub status: Option<String>,
    /// Filter by priority, e.g. "High"
    pub priority: Option<String>,
    /// Filter by size, e.g. "Small"
    pub size: Option<String>,
    /// Max results to return
    pub limit: Option<u64>,
    /// When true, omit issues blocked by at least one non-Done dependency
    /// (Canceled dependencies still count as active blockers)
    pub exclude_blocked: Option<bool>,
    /// Filter by submitter user token, e.g. "U_1"
    pub submitted_by: Option<String>,
    /// Filter by assigned user token, e.g. "U_2"
    pub assigned_to: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateIssueParams {
    /// Project slug, e.g. "issueboss"
    pub project_slug: String,
    /// Issue title
    pub title: String,
    /// Issue description (optional)
    pub description: Option<String>,
    /// Priority: Urgent, High, Medium, Low (defaults to Medium)
    pub priority: Option<String>,
    /// Size: XS, Small, Medium, Large
    pub size: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateIssueParams {
    /// Issue slug, e.g. "IB-5"
    pub slug: String,
    /// New title (optional)
    pub title: Option<String>,
    /// New description (optional)
    pub description: Option<String>,
    /// New priority (optional)
    pub priority: Option<String>,
    /// New size (optional)
    pub size: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TransitionIssueParams {
    /// Issue slug, e.g. "IB-5"
    pub slug: String,
    /// Target status. Valid values: TriageNeeded, TriageInProgress,
    /// TriageReview, ResearchNeeded, ResearchInProgress, ResearchReview,
    /// SpecNeeded, SpecInProgress, SpecReview,
    /// PlanNeeded, PlanInProgress, PlanReview,
    /// DevNeeded, DevInProgress, DevReview,
    /// Done, Backlog, Canceled
    pub new_status: String,
    /// Optional reason for the transition, recorded in the StatusTransition
    /// artifact
    pub reason: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddArtifactParams {
    /// Issue slug, e.g. "IB-42"
    issue_slug: String,
    /// Artifact kind: TriageResult, Spec, Research, Plan, ResearchTopic,
    /// Comment, Handoff
    kind: String,
    /// Artifact slug for multi-instance kinds (ResearchTopic, Research,
    /// Comment). Lowercase letters, digits, and hyphens only. Not required
    /// for singleton kinds (TriageResult, Spec, Plan) — those are
    /// auto-assigned.
    artifact_slug: Option<String>,
    /// JSON body — schema depends on kind (see artifact lifecycle design)
    body: serde_json::Value,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct UpdateArtifactParams {
    /// Issue slug, e.g. "IB-42"
    issue_slug: String,
    /// Artifact slug, e.g. "triage" or "my-comment"
    artifact_slug: String,
    /// Updated JSON body
    body: serde_json::Value,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct RemoveArtifactParams {
    /// Issue slug, e.g. "IB-42"
    issue_slug: String,
    /// Artifact slug, e.g. "triage" or "my-comment"
    artifact_slug: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ListArtifactsParams {
    /// Issue slug, e.g. "IB-42"
    issue_slug: String,
    /// Optional list of kinds to filter by (e.g. ["Research", "ResearchTopic"])
    kinds: Option<Vec<String>>,
    /// When true, only return ResearchTopics with no corresponding Research
    /// artifact
    #[serde(default)]
    uncovered_only: bool,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MoveArtifactParams {
    /// Current path stored in artifact bodies (e.g.
    /// ".insights/specs/IB-1-spec.md"). Move the file on disk first, then
    /// call this tool.
    old_path: String,
    /// Replacement path. All artifacts referencing old_path will be updated to
    /// this value.
    new_path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct AddRelationshipParams {
    /// Issue slug, e.g. "IB-5"
    issue_slug: String,
    /// Related issue slug, e.g. "IB-3"
    related_slug: String,
    /// Relationship kind: "DependsOn" or "RelatedTo"
    kind: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct RemoveRelationshipParams {
    /// Issue slug, e.g. "IB-5"
    issue_slug: String,
    /// Related issue slug, e.g. "IB-3"
    related_slug: String,
    /// Relationship kind: "DependsOn" or "RelatedTo"
    kind: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ListRelationshipsParams {
    /// Issue slug, e.g. "IB-5"
    issue_slug: String,
}

// ---------------------------------------------------------------------------
// Error helper
// ---------------------------------------------------------------------------

fn map_core_err(e: ib_core::Error) -> McpError {
    match e {
        ib_core::Error::RepositoryError(ib_core::RepositoryError::NotFound) => McpError::invalid_params("not found", None),
        ib_core::Error::Validation(msg) => McpError::invalid_params(msg, None),
        ib_core::Error::GateFailure { condition, failing_tokens } => McpError::invalid_params(
            serde_json::json!({
                "error": "gate_failed",
                "condition": condition,
                "tokens": failing_tokens,
            })
            .to_string(),
            None,
        ),
        ib_core::Error::CycleDetected => McpError::invalid_params("adding this relationship would create a dependency cycle", None),
        ib_core::Error::AlreadyExists => McpError::invalid_params("relationship already exists", None),
        _ => McpError::internal_error("internal server error", None),
    }
}

fn serialize<T: serde::Serialize>(value: &T) -> Result<String, McpError> {
    serde_json::to_string(value).map_err(|e| McpError::internal_error(e.to_string(), None))
}

fn build_completeness(status: &IssueStatus, artifacts: &[IssueArtifact]) -> serde_json::Value {
    // Only completed Research artifacts count as covering a topic; cancelled ones
    // do not.
    let covered: std::collections::HashSet<String> = artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::Research && a.body.get("status").and_then(|v| v.as_str()) == Some("completed"))
        .filter_map(|a| a.body.get("topic_token").and_then(|v| v.as_str()).map(|s| s.to_owned()))
        .collect();
    serde_json::json!({
        "status": status.to_string(),
        "has_triage_result": artifacts.iter().any(|a| a.kind == ArtifactKind::TriageResult),
        "has_spec": artifacts.iter().any(|a| a.kind == ArtifactKind::Spec),
        "has_plan": artifacts.iter().any(|a| a.kind == ArtifactKind::Plan),
        "research_topic_count": artifacts.iter().filter(|a| a.kind == ArtifactKind::ResearchTopic).count(),
        "uncovered_research_topics": artifacts.iter()
            .filter(|a| a.kind == ArtifactKind::ResearchTopic)
            .filter(|a| !covered.contains(&a.token.to_string()))
            .count(),
    })
}

async fn resolve_user_ref(core: &Arc<CoreServices>, user_id: UserId) -> Result<UserRefMcp, McpError> {
    let token = UserToken::new(user_id).to_string();
    match core.user_service().find_by_id(user_id).await.map_err(map_core_err)? {
        Some(u) => Ok(UserRefMcp {
            token,
            username: u.username,
            full_name: u.full_name,
        }),
        None => Ok(UserRefMcp {
            token,
            username: "unknown".into(),
            full_name: "unknown".into(),
        }),
    }
}

async fn build_issue_summary_mcp(core: &Arc<CoreServices>, issue: ib_core::issue::Issue) -> Result<IssueSummaryMcp, McpError> {
    let submitter = resolve_user_ref(core, issue.submitter).await?;
    let assigned = match issue.assigned {
        Some(id) => Some(resolve_user_ref(core, id).await?),
        None => None,
    };
    Ok(IssueSummaryMcp {
        token: issue.token.to_string(),
        number: issue.number,
        title: issue.title,
        description: issue.description,
        status: issue.status.to_string(),
        priority: issue.priority.to_string(),
        size: issue.size.map(|s| s.to_string()),
        slug: issue.slug,
        submitter,
        assigned,
        created_at: issue.created_at.to_rfc3339(),
        updated_at: issue.updated_at.to_rfc3339(),
    })
}

// ---------------------------------------------------------------------------
// IssueBossServer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct IssueBossServer {
    core: Arc<CoreServices>,
    user: User,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for IssueBossServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IssueBossServer").field("user", &self.user.username).finish_non_exhaustive()
    }
}

impl IssueBossServer {
    pub fn new(core: Arc<CoreServices>, user: User) -> Self {
        Self {
            core,
            user,
            tool_router: Self::tool_router(),
        }
    }

    /// Check that the authenticated user holds `cap` for the given project.
    ///
    /// Admin and SuperAdmin users bypass the membership check. All other users
    /// must be project members and hold the required capability. Returns an
    /// opaque "not found" error on failure to avoid leaking project existence.
    async fn require_capability(&self, project_id: ProjectId, cap: Capability) -> Result<(), McpError> {
        if self.user.capabilities.has(Capability::Admin) || self.user.capabilities.has(Capability::SuperAdmin) {
            return Ok(());
        }
        let caps = self
            .core
            .project_service()
            .capabilities_for_user(project_id, self.user.id)
            .await
            .map_err(map_core_err)?;
        if caps.has(cap) {
            Ok(())
        } else {
            Err(McpError::invalid_params("not found", None))
        }
    }
}

#[tool_router]
impl IssueBossServer {
    #[tool(
        description = "List issues in a project. Use status, priority, and size filters to efficiently narrow results. For large projects with thousands of \
                       issues, always filter by status to avoid retrieving unnecessary data. project_slug is typically pre-configured (e.g. \"issueboss\")."
    )]
    async fn list_issues(&self, params: Parameters<ListIssuesParams>) -> Result<String, McpError> {
        let p = params.0;
        let project = self
            .core
            .project_service()
            .find_by_slug(&p.project_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("project '{}' not found", p.project_slug), None))?;
        self.require_capability(project.id, Capability::ViewIssues).await?;

        let filter = IssueFilter {
            status: p
                .status
                .as_deref()
                .map(|s| {
                    s.parse::<IssueStatus>()
                        .map_err(|_| McpError::invalid_params(format!("invalid status: {s}"), None))
                })
                .transpose()?,
            priority: p
                .priority
                .as_deref()
                .map(|s| {
                    s.parse::<IssuePriority>()
                        .map_err(|_| McpError::invalid_params(format!("invalid priority: {s}"), None))
                })
                .transpose()?,
            size: p
                .size
                .as_deref()
                .map(|s| s.parse::<IssueSize>().map_err(|_| McpError::invalid_params(format!("invalid size: {s}"), None)))
                .transpose()?,
            limit: p.limit,
            exclude_blocked: p.exclude_blocked,
            submitted_by: p
                .submitted_by
                .as_deref()
                .map(|s| {
                    UserToken::parse(s)
                        .map(|t| t.id())
                        .map_err(|_| McpError::invalid_params(format!("invalid user token: {s}"), None))
                })
                .transpose()?,
            assigned_to: p
                .assigned_to
                .as_deref()
                .map(|s| {
                    UserToken::parse(s)
                        .map(|t| t.id())
                        .map_err(|_| McpError::invalid_params(format!("invalid user token: {s}"), None))
                })
                .transpose()?,
        };

        let issues = self.core.issue_service().list_issues(project.id, filter).await.map_err(map_core_err)?;

        let mut summaries = Vec::with_capacity(issues.len());
        for issue in issues {
            summaries.push(build_issue_summary_mcp(&self.core, issue).await?);
        }
        serialize(&summaries)
    }

    #[tool(
        description = "Create a new issue in a project. Priority defaults to Medium if omitted. Size is optional. Use project_slug to identify the project."
    )]
    async fn create_issue(&self, params: Parameters<CreateIssueParams>) -> Result<String, McpError> {
        let p = params.0;
        let project = self
            .core
            .project_service()
            .find_by_slug(&p.project_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("project '{}' not found", p.project_slug), None))?;
        self.require_capability(project.id, Capability::CreateIssues).await?;

        let priority = p
            .priority
            .as_deref()
            .map(|s| {
                s.parse::<IssuePriority>()
                    .map_err(|_| McpError::invalid_params(format!("invalid priority: {s}"), None))
            })
            .transpose()?
            .unwrap_or(IssuePriority::Medium);

        let size = p
            .size
            .as_deref()
            .map(|s| s.parse::<IssueSize>().map_err(|_| McpError::invalid_params(format!("invalid size: {s}"), None)))
            .transpose()?;

        let new_issue = NewIssue::new(
            project.id,
            &project.prefix,
            p.title,
            p.description.unwrap_or_default(),
            priority,
            size,
            self.user.id,
        )
        .map_err(map_core_err)?;

        let issue = self.core.issue_service().create_issue(new_issue).await.map_err(map_core_err)?;
        serialize(&build_issue_summary_mcp(&self.core, issue).await?)
    }

    #[tool(description = "Update an existing issue's title, description, priority, or size.")]
    async fn update_issue(&self, params: Parameters<UpdateIssueParams>) -> Result<String, McpError> {
        let p = params.0;
        let mut issue = self
            .core
            .issue_service()
            .find_by_slug(&p.slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.slug), None))?;
        self.require_capability(issue.project_id, Capability::UpdateIssues).await?;

        if let Some(title) = p.title {
            issue.title = title;
        }
        if let Some(description) = p.description {
            issue.description = description;
        }
        if let Some(priority) = p.priority {
            issue.priority = priority
                .parse::<IssuePriority>()
                .map_err(|_| McpError::invalid_params(format!("invalid priority: {priority}"), None))?;
        }
        if let Some(size) = p.size {
            issue.size = Some(
                size.parse::<IssueSize>()
                    .map_err(|_| McpError::invalid_params(format!("invalid size: {size}"), None))?,
            );
        }

        let updated = self.core.issue_service().update_issue(issue).await.map_err(map_core_err)?;
        serialize(&build_issue_summary_mcp(&self.core, updated).await?)
    }

    #[tool(
        description = "Transition an issue to a new status. Pipeline: TriageNeeded → TriageInProgress → TriageReview → ResearchNeeded → ResearchInProgress → \
                       ResearchReview → SpecNeeded → SpecInProgress → SpecReview → PlanNeeded → PlanInProgress → PlanReview → DevNeeded → DevInProgress → \
                       DevReview → Done. Backlog and Canceled reachable from most states. Gated transitions require artifact prerequisites."
    )]
    async fn transition_issue(&self, params: Parameters<TransitionIssueParams>) -> Result<String, McpError> {
        let p = params.0;
        let new_status = p
            .new_status
            .parse::<IssueStatus>()
            .map_err(|_| McpError::invalid_params(format!("invalid status: {}", p.new_status), None))?;

        let issue = self
            .core
            .issue_service()
            .find_by_slug(&p.slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.slug), None))?;
        self.require_capability(issue.project_id, Capability::UpdateIssues).await?;

        let updated = self
            .core
            .issue_service()
            .transition_issue(issue.token, new_status, p.reason, self.user.id)
            .await
            .map_err(map_core_err)?;

        let artifacts = self
            .core
            .artifact_service()
            .list_artifacts(updated.id, None, false)
            .await
            .map_err(map_core_err)?;

        let completeness = build_completeness(&updated.status, &artifacts);
        serialize(&TransitionIssueMcp {
            issue: build_issue_summary_mcp(&self.core, updated).await?,
            completeness,
        })
    }

    #[tool(
        description = "Add an artifact to an issue. Kinds: TriageResult (path required), Spec (path required), Research (topic_token, status, path when \
                       completed), Plan (path required), ResearchTopic (description or path), Comment (text), Handoff (path required). StatusTransition is \
                       system-generated only."
    )]
    async fn add_artifact(&self, params: Parameters<AddArtifactParams>) -> Result<String, McpError> {
        let p = params.0;
        let kind = p
            .kind
            .parse::<ArtifactKind>()
            .map_err(|_| McpError::invalid_params(format!("invalid kind: {}", p.kind), None))?;
        let issue = self
            .core
            .issue_service()
            .find_by_slug(&p.issue_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.issue_slug), None))?;
        self.require_capability(issue.project_id, Capability::UpdateIssues).await?;
        let artifact = self
            .core
            .artifact_service()
            .add_artifact(NewArtifact {
                issue_id: issue.id,
                kind,
                slug: p.artifact_slug,
                body: p.body,
                created_by: self.user.token.to_string(),
            })
            .await
            .map_err(map_core_err)?;
        serialize(&ArtifactMcp::from_artifact(artifact))
    }

    #[tool(description = "Update an artifact's body. StatusTransition artifacts and file path fields are immutable.")]
    async fn update_artifact(&self, params: Parameters<UpdateArtifactParams>) -> Result<String, McpError> {
        let p = params.0;
        let issue = self
            .core
            .issue_service()
            .find_by_slug(&p.issue_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.issue_slug), None))?;
        self.require_capability(issue.project_id, Capability::UpdateIssues).await?;
        let artifact = self
            .core
            .artifact_service()
            .update_artifact(issue.id, &p.artifact_slug, p.body)
            .await
            .map_err(map_core_err)?;
        serialize(&ArtifactMcp::from_artifact(artifact))
    }

    #[tool(description = "Remove an artifact. For ResearchTopics, prefer adding a Research artifact with status 'cancelled' instead of removing.")]
    async fn remove_artifact(&self, params: Parameters<RemoveArtifactParams>) -> Result<String, McpError> {
        let p = params.0;
        let issue = self
            .core
            .issue_service()
            .find_by_slug(&p.issue_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.issue_slug), None))?;
        self.require_capability(issue.project_id, Capability::UpdateIssues).await?;
        self.core
            .artifact_service()
            .remove_artifact(issue.id, &p.artifact_slug)
            .await
            .map_err(map_core_err)?;
        serialize(&serde_json::json!({"ok": true}))
    }

    #[tool(
        description = "List artifacts for an issue. Filter by kinds (e.g. [\"Research\", \"ResearchTopic\"]); use uncovered_only=true to find ResearchTopics \
                       not yet addressed by a Research artifact."
    )]
    async fn list_artifacts(&self, params: Parameters<ListArtifactsParams>) -> Result<String, McpError> {
        let p = params.0;
        let issue = self
            .core
            .issue_service()
            .find_by_slug(&p.issue_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.issue_slug), None))?;
        self.require_capability(issue.project_id, Capability::ViewIssues).await?;
        let kinds = p
            .kinds
            .map(|ks| {
                ks.into_iter()
                    .map(|k| {
                        k.parse::<ArtifactKind>()
                            .map_err(|_| McpError::invalid_params(format!("invalid kind: {k}"), None))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?;
        let artifacts = self
            .core
            .artifact_service()
            .list_artifacts(issue.id, kinds, p.uncovered_only)
            .await
            .map_err(map_core_err)?;
        serialize(&artifacts.into_iter().map(ArtifactMcp::from_artifact).collect::<Vec<_>>())
    }

    #[tool(
        description = "Move/rename artifact file paths. Finds all artifacts across all issues whose body path equals old_path and updates them to new_path \
                       atomically. Move the file on disk first — the server does not access the filesystem. Returns the count and list of updated artifacts."
    )]
    async fn move_artifact(&self, params: Parameters<MoveArtifactParams>) -> Result<String, McpError> {
        if !self.user.capabilities.has(Capability::Admin) && !self.user.capabilities.has(Capability::SuperAdmin) {
            return Err(McpError::invalid_params("not found", None));
        }
        let p = params.0;
        let updated = self
            .core
            .artifact_service()
            .move_artifact(&p.old_path, &p.new_path)
            .await
            .map_err(map_core_err)?;

        // Resolve issue_id → issue_slug; cache to avoid duplicate lookups for
        // artifacts on the same issue
        let mut slug_cache: HashMap<IssueId, String> = HashMap::new();
        let mut artifacts = Vec::with_capacity(updated.len());
        for artifact in &updated {
            let issue_slug = if let Some(slug) = slug_cache.get(&artifact.issue_id) {
                slug.clone()
            } else {
                let slug = self
                    .core
                    .issue_service()
                    .find_by_id(artifact.issue_id)
                    .await
                    .map_err(map_core_err)?
                    .map(|i| i.slug)
                    .unwrap_or_else(|| format!("#{}", artifact.issue_id));
                slug_cache.insert(artifact.issue_id, slug.clone());
                slug
            };
            artifacts.push(MovedArtifactMcp {
                issue_slug,
                slug: artifact.slug.clone(),
                kind: artifact.kind.to_string(),
            });
        }

        serialize(&MoveArtifactResultMcp {
            updated: artifacts.len(),
            artifacts,
        })
    }

    #[tool(
        description = "Add a relationship between two issues in the same project. kind must be 'DependsOn' (issue_slug depends on related_slug) or \
                       'RelatedTo' (symmetric). DependsOn relationships are validated for cycles. Requires UpdateIssues capability."
    )]
    async fn add_relationship(&self, params: Parameters<AddRelationshipParams>) -> Result<String, McpError> {
        let p = params.0;
        let kind = p
            .kind
            .parse::<ib_core::relationship::RelationshipKind>()
            .map_err(|_| McpError::invalid_params(format!("invalid kind: {}", p.kind), None))?;
        let issue = self
            .core
            .issue_service()
            .find_by_slug(&p.issue_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.issue_slug), None))?;
        self.require_capability(issue.project_id, Capability::UpdateIssues).await?;
        self.core
            .relationship_service()
            .add_relationship(&p.issue_slug, &p.related_slug, kind)
            .await
            .map_err(map_core_err)?;
        serialize(&serde_json::json!({
            "ok": true,
            "from": p.issue_slug,
            "to": p.related_slug,
            "kind": p.kind,
        }))
    }

    #[tool(
        description = "Remove a relationship between two issues. kind must be 'DependsOn' or 'RelatedTo'. Returns ok=true if found and removed, ok=false if \
                       not found. Requires UpdateIssues capability."
    )]
    async fn remove_relationship(&self, params: Parameters<RemoveRelationshipParams>) -> Result<String, McpError> {
        let p = params.0;
        let kind = p
            .kind
            .parse::<ib_core::relationship::RelationshipKind>()
            .map_err(|_| McpError::invalid_params(format!("invalid kind: {}", p.kind), None))?;
        let issue = self
            .core
            .issue_service()
            .find_by_slug(&p.issue_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.issue_slug), None))?;
        self.require_capability(issue.project_id, Capability::UpdateIssues).await?;
        let removed = self
            .core
            .relationship_service()
            .remove_relationship(&p.issue_slug, &p.related_slug, kind)
            .await
            .map_err(map_core_err)?;
        serialize(&serde_json::json!({ "ok": removed }))
    }

    #[tool(
        description = "List all relationships for an issue. Returns depends_on (issues this issue depends on), blocks (issues depending on this issue), and \
                       related_to (symmetric relationships). Requires ViewIssues capability."
    )]
    async fn list_relationships(&self, params: Parameters<ListRelationshipsParams>) -> Result<String, McpError> {
        let p = params.0;
        let issue = self
            .core
            .issue_service()
            .find_by_slug(&p.issue_slug)
            .await
            .map_err(map_core_err)?
            .ok_or_else(|| McpError::invalid_params(format!("issue '{}' not found", p.issue_slug), None))?;
        self.require_capability(issue.project_id, Capability::ViewIssues).await?;
        let rels = self.core.relationship_service().list_for_issue(issue.id).await.map_err(map_core_err)?;
        serialize(&IssueRelationshipsMcp::from(rels))
    }
}

impl IssueBossServer {
    /// Inner implementation for `read_resource` — testable without a
    /// `RequestContext`.
    async fn read_resource_inner(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        if uri == "issueboss://projects" {
            let projects = self.core.project_service().list_for_user(self.user.id).await.map_err(map_core_err)?;
            let summaries: Vec<ProjectSummaryMcp> = projects.into_iter().map(Into::into).collect();
            let json = serialize(&summaries)?;
            Ok(ReadResourceResult::new(vec![
                ResourceContents::text(json, uri).with_mime_type("application/json"),
            ]))
        } else if let Some(slug) = uri.strip_prefix("issueboss://issues/") {
            let issue = self
                .core
                .issue_service()
                .find_by_slug(slug)
                .await
                .map_err(map_core_err)?
                .ok_or_else(|| McpError::invalid_params(format!("issue '{slug}' not found"), None))?;
            self.require_capability(issue.project_id, Capability::ViewIssues).await?;
            let relationships = self.core.relationship_service().list_for_issue(issue.id).await.map_err(map_core_err)?;
            let issue_summary = build_issue_summary_mcp(&self.core, issue).await?;
            let json = serialize(&serde_json::json!({
                "issue": issue_summary,
                "relationships": IssueRelationshipsMcp::from(relationships),
            }))?;
            Ok(ReadResourceResult::new(vec![
                ResourceContents::text(json, uri).with_mime_type("application/json"),
            ]))
        } else {
            Err(McpError::invalid_params(format!("Unknown resource URI: {uri}"), None))
        }
    }
}

#[tool_handler]
impl ServerHandler for IssueBossServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().enable_resources().build()).with_instructions(
            "IssueBoss issue tracker. The project slug is pre-configured — use it with list_issues to find issues. Filter by status (e.g. TriageNeeded, \
             DevInProgress) to efficiently query large projects. Use transition_issue to move issues through the pipeline. Resources: issueboss://projects \
             lists all accessible projects; issueboss://issues/{slug} reads a single issue (e.g. IB-5).",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(vec![Annotated::new(
            RawResource::new("issueboss://projects", "Projects")
                .with_description("All projects the authenticated user is a member of")
                .with_mime_type("application/json"),
            None,
        )]))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult::with_all_items(vec![Annotated::new(
            RawResourceTemplate::new("issueboss://issues/{slug}", "Issue by Slug").with_description("A single issue by slug, e.g. IB-5"),
            None,
        )]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        self.read_resource_inner(&request.uri).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use ib_core::{
        api_key::MockApiKeyRepository,
        issue::{IssuePriority, IssueStatus, IssueToken, MockIssueRepository},
        project::{MockProjectMemberRepository, MockProjectRepository, Project, ProjectMember, ProjectToken},
        types::Capabilities,
        user::{MockUserRepository, User, UserToken},
    };

    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

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
        fake_user_with_caps(id, Capabilities(vec![ib_core::types::Capability::Admin]))
    }

    fn fake_user_with_caps(id: u64, capabilities: Capabilities) -> User {
        User {
            id,
            token: UserToken::new(id),
            username: "alice".to_owned(),
            full_name: "Alice".to_owned(),
            password_hash: "h".to_owned(),
            email_address: "alice@example.com".to_owned(),
            capabilities,
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
            status: IssueStatus::TriageNeeded,
            priority: IssuePriority::Medium,
            size: None,
            slug: format!("TP-{number}"),
            submitter: 1,
            assigned: None,
            version: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_core(project_repo: MockProjectRepository, issue_repo: MockIssueRepository) -> Arc<ib_core::CoreServices> {
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

    fn make_core_with_artifacts(
        project_repo: MockProjectRepository,
        issue_repo: MockIssueRepository,
        artifact_repo: ib_core::artifact::MockArtifactRepository,
    ) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(MockUserRepository::new()))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .artifact_repository(Arc::new(artifact_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    /// Like `make_core` but registers the given `User` in the mock user repo
    /// so that `resolve_user_ref` calls succeed when building
    /// `IssueSummaryMcp`.
    fn make_core_with_user(project_repo: MockProjectRepository, issue_repo: MockIssueRepository, user: ib_core::user::User) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(user_repo))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    fn make_core_with_members(
        project_repo: MockProjectRepository,
        issue_repo: MockIssueRepository,
        user_repo: MockUserRepository,
        member_repo: ib_core::project::MockProjectMemberRepository,
    ) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(user_repo))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(project_repo))
                .project_member_repository(Arc::new(member_repo))
                .issue_repository(Arc::new(issue_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    fn make_core_with_artifacts_and_user(
        project_repo: MockProjectRepository,
        issue_repo: MockIssueRepository,
        artifact_repo: ib_core::artifact::MockArtifactRepository,
        user: ib_core::user::User,
    ) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(user_repo))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .artifact_repository(Arc::new(artifact_repo))
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    fn make_core_with_relationships(
        project_repo: MockProjectRepository,
        issue_repo: MockIssueRepository,
        rel_repo: ib_core::relationship::MockIssueRelationshipRepository,
    ) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(MockUserRepository::new()))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .relationship_repository(Arc::new(rel_repo) as Arc<dyn ib_core::relationship::IssueRelationshipRepository>)
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    fn make_core_with_relationships_and_user(
        project_repo: MockProjectRepository,
        issue_repo: MockIssueRepository,
        rel_repo: ib_core::relationship::MockIssueRelationshipRepository,
        user: ib_core::user::User,
    ) -> Arc<ib_core::CoreServices> {
        use ib_core::repository::testing::default_repository_service_builder;
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let repo_svc = Arc::new(
            default_repository_service_builder()
                .user_repository(Arc::new(user_repo))
                .api_key_repository(Arc::new(MockApiKeyRepository::new()))
                .project_repository(Arc::new(project_repo))
                .issue_repository(Arc::new(issue_repo))
                .relationship_repository(Arc::new(rel_repo) as Arc<dyn ib_core::relationship::IssueRelationshipRepository>)
                .build()
                .unwrap(),
        );
        ib_core::create_services(repo_svc)
    }

    // -----------------------------------------------------------------------
    // list_issues
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn list_issues_happy_path() {
        let project = fake_project(1, "myapp");
        let issue = fake_issue(10, 1, 1);

        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
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

        let core = make_core_with_user(project_repo, issue_repo, fake_user(1));
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .list_issues(Parameters(ListIssuesParams {
                project_slug: "myapp".to_string(),
                status: None,
                priority: None,
                size: None,
                limit: None,
                exclude_blocked: None,
                submitted_by: None,
                assigned_to: None,
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["slug"], "TP-1");
    }

    #[tokio::test]
    async fn list_issues_project_not_found() {
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        let issue_repo = MockIssueRepository::new();

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .list_issues(Parameters(ListIssuesParams {
                project_slug: "nope".to_string(),
                status: None,
                priority: None,
                size: None,
                limit: None,
                exclude_blocked: None,
                submitted_by: None,
                assigned_to: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_issues_invalid_status() {
        let project = fake_project(1, "myapp");
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let issue_repo = MockIssueRepository::new();

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .list_issues(Parameters(ListIssuesParams {
                project_slug: "myapp".to_string(),
                status: Some("NotAStatus".to_string()),
                priority: None,
                size: None,
                limit: None,
                exclude_blocked: None,
                submitted_by: None,
                assigned_to: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_issues_non_member_returns_error() {
        let project = fake_project(1, "myapp");
        let user = fake_user_with_caps(1, Capabilities::default());

        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));

        let core = make_core_with_members(project_repo, MockIssueRepository::new(), user_repo, member_repo);
        let server = IssueBossServer::new(core, user);

        let result = server
            .list_issues(Parameters(ListIssuesParams {
                project_slug: "myapp".to_string(),
                status: None,
                priority: None,
                size: None,
                limit: None,
                exclude_blocked: None,
                submitted_by: None,
                assigned_to: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_issues_insufficient_capability_returns_error() {
        let project = fake_project(1, "myapp");
        let user = fake_user_with_caps(1, Capabilities::default());

        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        // Member exists but has no capabilities
        let mut member_repo = MockProjectMemberRepository::new();
        {
            let member = ProjectMember {
                project_id: 1,
                user_id: 1,
                capabilities: Capabilities::default(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            member_repo.expect_find().returning(move |_, _, _| {
                let m = member.clone();
                Box::pin(async move { Ok(Some(m)) })
            });
        }

        let core = make_core_with_members(project_repo, MockIssueRepository::new(), user_repo, member_repo);
        let server = IssueBossServer::new(core, user);

        let result = server
            .list_issues(Parameters(ListIssuesParams {
                project_slug: "myapp".to_string(),
                status: None,
                priority: None,
                size: None,
                limit: None,
                exclude_blocked: None,
                submitted_by: None,
                assigned_to: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_issue_non_member_returns_error() {
        let project = fake_project(1, "myapp");
        let user = fake_user_with_caps(1, Capabilities::default());

        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));

        let core = make_core_with_members(project_repo, MockIssueRepository::new(), user_repo, member_repo);
        let server = IssueBossServer::new(core, user);

        let result = server
            .create_issue(Parameters(CreateIssueParams {
                project_slug: "myapp".to_string(),
                title: "Test".to_string(),
                description: None,
                priority: None,
                size: None,
            }))
            .await;

        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // create_issue
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn create_issue_happy_path() {
        let project = fake_project(1, "myapp");
        let issue = fake_issue(20, 1, 2);

        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        {
            let p = project.clone();
            // create_issue internally calls find_by_id to verify the project exists
            project_repo.expect_find_by_id().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        {
            // create_issue calls increment_issue_counter to get the next issue number
            project_repo.expect_increment_issue_counter().returning(|_, _| Box::pin(async { Ok(2u32) }));
        }
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_create().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(i) })
            });
        }

        let core = make_core_with_user(project_repo, issue_repo, fake_user(1));
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .create_issue(Parameters(CreateIssueParams {
                project_slug: "myapp".to_string(),
                title: "Test issue".to_string(),
                description: None,
                priority: None,
                size: None,
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["slug"], "TP-2");
    }

    #[tokio::test]
    async fn issue_summary_mcp_includes_submitter_token_and_unknown_for_deleted_user() {
        use ib_core::user::MockUserRepository;
        // fake_issue has submitter: 1, assigned: None
        let issue = fake_issue(10, 1, 1);

        let mut user_repo = MockUserRepository::new();
        // find_by_id returns None → deleted user → expect "unknown"
        user_repo.expect_find_by_id().returning(|_, _| Box::pin(async { Ok(None) }));

        use ib_core::repository::testing::default_repository_service_builder;
        let repo_svc = std::sync::Arc::new(
            default_repository_service_builder()
                .user_repository(std::sync::Arc::new(user_repo))
                .build()
                .unwrap(),
        );
        let core = ib_core::create_services(repo_svc);

        let summary = build_issue_summary_mcp(&core, issue).await.unwrap();
        assert_eq!(summary.submitter.username, "unknown");
        assert_eq!(summary.submitter.full_name, "unknown");
        assert!(summary.submitter.token.starts_with("U_"));
        assert!(summary.assigned.is_none());
    }

    #[tokio::test]
    async fn create_issue_project_not_found() {
        let mut project_repo = MockProjectRepository::new();
        project_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        let issue_repo = MockIssueRepository::new();

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .create_issue(Parameters(CreateIssueParams {
                project_slug: "nope".to_string(),
                title: "Test".to_string(),
                description: None,
                priority: None,
                size: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn create_issue_invalid_priority() {
        let project = fake_project(1, "myapp");
        let mut project_repo = MockProjectRepository::new();
        {
            let p = project.clone();
            project_repo.expect_find_by_slug().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(Some(p)) })
            });
        }
        let issue_repo = MockIssueRepository::new();

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .create_issue(Parameters(CreateIssueParams {
                project_slug: "myapp".to_string(),
                title: "Test".to_string(),
                description: None,
                priority: Some("NotAPriority".to_string()),
                size: None,
            }))
            .await;

        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // update_issue
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn update_issue_happy_path() {
        let issue = fake_issue(10, 1, 1);
        let updated = {
            let mut i = issue.clone();
            i.title = "Updated title".to_string();
            i
        };

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        {
            let u = updated.clone();
            issue_repo.expect_update().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(u) })
            });
        }

        let core = make_core_with_user(project_repo, issue_repo, fake_user(1));
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .update_issue(Parameters(UpdateIssueParams {
                slug: "TP-1".to_string(),
                title: Some("Updated title".to_string()),
                description: None,
                priority: None,
                size: None,
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["title"], "Updated title");
    }

    #[tokio::test]
    async fn update_issue_not_found() {
        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .update_issue(Parameters(UpdateIssueParams {
                slug: "TP-999".to_string(),
                title: Some("Title".to_string()),
                description: None,
                priority: None,
                size: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_issue_invalid_priority() {
        let issue = fake_issue(10, 1, 1);
        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .update_issue(Parameters(UpdateIssueParams {
                slug: "TP-1".to_string(),
                title: None,
                description: None,
                priority: Some("NotAPriority".to_string()),
                size: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_issue_non_member_returns_error() {
        let issue = fake_issue(10, 1, 1);
        let user = fake_user_with_caps(1, Capabilities::default());
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        let core = make_core_with_members(MockProjectRepository::new(), issue_repo, user_repo, member_repo);
        let result = IssueBossServer::new(core, user)
            .update_issue(Parameters(UpdateIssueParams {
                slug: "TP-1".to_string(),
                title: Some("New title".to_string()),
                description: None,
                priority: None,
                size: None,
            }))
            .await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // transition_issue
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn transition_issue_happy_path() {
        use ib_core::artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        };
        let issue = fake_issue(10, 1, 1); // TriageNeeded
        let transitioned = {
            let mut i = issue.clone();
            i.status = IssueStatus::TriageInProgress;
            i.assigned = Some(1);
            i
        };

        let project_repo = MockProjectRepository::new();
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
                .withf(|_, i| i.status == IssueStatus::TriageInProgress && i.assigned == Some(1))
                .returning(move |_, _| {
                    let t = t.clone();
                    Box::pin(async move { Ok(t) })
                });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));
        artifact_repo.expect_create().returning(|_, _| {
            Box::pin(async move {
                Ok(IssueArtifact {
                    id: 999,
                    token: ArtifactToken::new(999),
                    issue_id: 10,
                    kind: ArtifactKind::StatusTransition,
                    slug: None,
                    body: serde_json::json!({}),
                    created_by: "system".into(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            })
        });

        let core = make_core_with_artifacts_and_user(project_repo, issue_repo, artifact_repo, fake_user(1));
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .transition_issue(Parameters(TransitionIssueParams {
                slug: "TP-1".to_string(),
                new_status: "TriageInProgress".to_string(),
                reason: None,
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["issue"]["status"], "TriageInProgress");
        assert!(json["completeness"].is_object());
    }

    #[tokio::test]
    async fn transition_issue_not_found() {
        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .transition_issue(Parameters(TransitionIssueParams {
                slug: "TP-999".to_string(),
                new_status: "TriageInProgress".to_string(),
                reason: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn transition_issue_invalid_status() {
        let project_repo = MockProjectRepository::new();
        let issue_repo = MockIssueRepository::new();

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .transition_issue(Parameters(TransitionIssueParams {
                slug: "TP-1".to_string(),
                new_status: "NotAStatus".to_string(),
                reason: None,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn transition_issue_non_member_returns_error() {
        let issue = fake_issue(10, 1, 1);
        let user = fake_user_with_caps(1, Capabilities::default());
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        let core = make_core_with_members(MockProjectRepository::new(), issue_repo, user_repo, member_repo);
        let result = IssueBossServer::new(core, user)
            .transition_issue(Parameters(TransitionIssueParams {
                slug: "TP-1".to_string(),
                new_status: "TriageInProgress".to_string(),
                reason: None,
            }))
            .await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // read_resource — projects
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn read_resource_projects_happy_path() {
        let user = fake_user(1);
        let projects = vec![fake_project(1, "app1"), fake_project(2, "app2")];

        let mut project_repo = MockProjectRepository::new();
        {
            let p = projects.clone();
            project_repo.expect_list_for_user().returning(move |_, _| {
                let p = p.clone();
                Box::pin(async move { Ok(p) })
            });
        }
        let issue_repo = MockIssueRepository::new();

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, user);

        let result = server.read_resource_inner("issueboss://projects").await;

        assert!(result.is_ok());
        let ReadResourceResult { contents, .. } = result.unwrap();
        assert_eq!(contents.len(), 1);
        if let ResourceContents::TextResourceContents { text, .. } = &contents[0] {
            let json: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(json.as_array().unwrap().len(), 2);
        } else {
            panic!("expected text resource contents");
        }
    }

    // -----------------------------------------------------------------------
    // read_resource — issues/{slug}
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn read_resource_issue_by_slug_happy_path() {
        use ib_core::relationship::{MockIssueRelationshipRepository, model::IssueRelationships};

        let issue = fake_issue(10, 1, 1);

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut rel_repo = MockIssueRelationshipRepository::new();
        rel_repo
            .expect_list_for_issue()
            .returning(|_, _| Box::pin(async { Ok(IssueRelationships::default()) }));

        let core = make_core_with_relationships_and_user(project_repo, issue_repo, rel_repo, fake_user(1));
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server.read_resource_inner("issueboss://issues/TP-1").await;

        assert!(result.is_ok());
        let ReadResourceResult { contents, .. } = result.unwrap();
        assert_eq!(contents.len(), 1);
        if let ResourceContents::TextResourceContents { text, .. } = &contents[0] {
            let json: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(json["issue"]["slug"], "TP-1");
            assert!(json["relationships"].is_object());
        } else {
            panic!("expected text resource contents");
        }
    }

    #[tokio::test]
    async fn read_resource_unknown_uri_returns_error() {
        let project_repo = MockProjectRepository::new();
        let issue_repo = MockIssueRepository::new();

        let core = make_core(project_repo, issue_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server.read_resource_inner("issueboss://unknown").await;

        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // add_artifact
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn add_artifact_happy_path() {
        use ib_core::artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        };
        let issue = fake_issue(10, 1, 1);
        let artifact = IssueArtifact {
            id: 1,
            token: ArtifactToken::new(1),
            issue_id: 10,
            kind: ArtifactKind::Comment,
            slug: None,
            body: serde_json::json!({"text": "hello"}),
            created_by: "U_1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let a = artifact.clone();
            artifact_repo.expect_create().returning(move |_, _| {
                let a = a.clone();
                Box::pin(async move { Ok(a) })
            });
        }

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .add_artifact(Parameters(AddArtifactParams {
                issue_slug: "TP-1".to_string(),
                kind: "Comment".to_string(),
                artifact_slug: Some("my-comment".to_string()),
                body: serde_json::json!({"text": "hello"}),
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["kind"], "Comment");
        assert_eq!(json["slug"], serde_json::json!(null));
    }

    #[tokio::test]
    async fn add_artifact_issue_not_found() {
        use ib_core::artifact::MockArtifactRepository;
        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        let artifact_repo = MockArtifactRepository::new();

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .add_artifact(Parameters(AddArtifactParams {
                issue_slug: "TP-999".to_string(),
                kind: "Comment".to_string(),
                artifact_slug: None,
                body: serde_json::json!({"text": "hello"}),
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn add_artifact_invalid_kind() {
        use ib_core::artifact::MockArtifactRepository;
        let project_repo = MockProjectRepository::new();
        let issue_repo = MockIssueRepository::new();
        let artifact_repo = MockArtifactRepository::new();

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .add_artifact(Parameters(AddArtifactParams {
                issue_slug: "TP-1".to_string(),
                kind: "NotAKind".to_string(),
                artifact_slug: None,
                body: serde_json::json!({}),
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn add_artifact_non_member_returns_error() {
        let issue = fake_issue(10, 1, 1);
        let user = fake_user_with_caps(1, Capabilities::default());
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        let core = make_core_with_members(MockProjectRepository::new(), issue_repo, user_repo, member_repo);
        let result = IssueBossServer::new(core, user)
            .add_artifact(Parameters(AddArtifactParams {
                issue_slug: "TP-1".to_string(),
                kind: "Comment".to_string(),
                artifact_slug: None,
                body: serde_json::json!({"text": "hello"}),
            }))
            .await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // update_artifact
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn update_artifact_happy_path() {
        use ib_core::artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        };
        let issue = fake_issue(10, 1, 1);
        let original = IssueArtifact {
            id: 1,
            token: ArtifactToken::new(1),
            issue_id: 10,
            kind: ArtifactKind::Comment,
            slug: Some("my-comment".to_string()),
            body: serde_json::json!({"text": "old"}),
            created_by: "U_1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let updated = IssueArtifact {
            body: serde_json::json!({"text": "updated"}),
            ..original.clone()
        };

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let a = original.clone();
            artifact_repo.expect_find_by_slug().returning(move |_, _, _| {
                let a = a.clone();
                Box::pin(async move { Ok(Some(a)) })
            });
        }
        {
            let u = updated.clone();
            artifact_repo.expect_update().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(u) })
            });
        }

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .update_artifact(Parameters(UpdateArtifactParams {
                issue_slug: issue.slug.clone(),
                artifact_slug: "my-comment".to_string(),
                body: serde_json::json!({"text": "updated"}),
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["body"]["text"], "updated");
        assert_eq!(json["slug"], "my-comment");
    }

    #[tokio::test]
    async fn update_artifact_issue_not_found() {
        use ib_core::artifact::MockArtifactRepository;
        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        let artifact_repo = MockArtifactRepository::new();

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .update_artifact(Parameters(UpdateArtifactParams {
                issue_slug: "NOTEXIST-1".to_string(),
                artifact_slug: "triage".to_string(),
                body: serde_json::json!({"text": "x"}),
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_artifact_non_member_returns_error() {
        let issue = fake_issue(10, 1, 1);
        let user = fake_user_with_caps(1, Capabilities::default());
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        let core = make_core_with_members(MockProjectRepository::new(), issue_repo, user_repo, member_repo);
        let result = IssueBossServer::new(core, user)
            .update_artifact(Parameters(UpdateArtifactParams {
                issue_slug: "TP-1".to_string(),
                artifact_slug: "comment".to_string(),
                body: serde_json::json!({"text": "updated"}),
            }))
            .await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // remove_artifact
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn remove_artifact_happy_path() {
        use ib_core::artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        };
        let issue = fake_issue(10, 1, 1);
        let artifact = IssueArtifact {
            id: 1,
            token: ArtifactToken::new(1),
            issue_id: 10,
            kind: ArtifactKind::Comment,
            slug: Some("my-comment".to_string()),
            body: serde_json::json!({"text": "hello"}),
            created_by: "U_1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let a = artifact.clone();
            artifact_repo.expect_find_by_slug().returning(move |_, _, _| {
                let a = a.clone();
                Box::pin(async move { Ok(Some(a)) })
            });
        }
        artifact_repo.expect_delete().returning(|_, _| Box::pin(async { Ok(()) }));

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .remove_artifact(Parameters(RemoveArtifactParams {
                issue_slug: issue.slug.clone(),
                artifact_slug: "my-comment".to_string(),
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["ok"], true);
    }

    #[tokio::test]
    async fn remove_artifact_issue_not_found() {
        use ib_core::artifact::MockArtifactRepository;
        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        let artifact_repo = MockArtifactRepository::new();

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .remove_artifact(Parameters(RemoveArtifactParams {
                issue_slug: "NOTEXIST-1".to_string(),
                artifact_slug: "triage".to_string(),
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn remove_artifact_non_member_returns_error() {
        let issue = fake_issue(10, 1, 1);
        let user = fake_user_with_caps(1, Capabilities::default());
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        let core = make_core_with_members(MockProjectRepository::new(), issue_repo, user_repo, member_repo);
        let result = IssueBossServer::new(core, user)
            .remove_artifact(Parameters(RemoveArtifactParams {
                issue_slug: "TP-1".to_string(),
                artifact_slug: "comment".to_string(),
            }))
            .await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // list_artifacts
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn list_artifacts_happy_path() {
        use ib_core::artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        };
        let issue = fake_issue(10, 1, 1);
        let artifact = IssueArtifact {
            id: 1,
            token: ArtifactToken::new(1),
            issue_id: 10,
            kind: ArtifactKind::Comment,
            slug: None,
            body: serde_json::json!({"text": "hello"}),
            created_by: "U_1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let a = artifact.clone();
            artifact_repo.expect_list().returning(move |_, _, _| {
                let a = a.clone();
                Box::pin(async move { Ok(vec![a]) })
            });
        }

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .list_artifacts(Parameters(ListArtifactsParams {
                issue_slug: "TP-1".to_string(),
                kinds: None,
                uncovered_only: false,
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["kind"], "Comment");
    }

    #[tokio::test]
    async fn list_artifacts_issue_not_found() {
        use ib_core::artifact::MockArtifactRepository;
        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        issue_repo.expect_find_by_slug().returning(|_, _| Box::pin(async { Ok(None) }));
        let artifact_repo = MockArtifactRepository::new();

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .list_artifacts(Parameters(ListArtifactsParams {
                issue_slug: "TP-999".to_string(),
                kinds: None,
                uncovered_only: false,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_artifacts_invalid_kind() {
        use ib_core::artifact::MockArtifactRepository;
        let issue = fake_issue(10, 1, 1);

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let artifact_repo = MockArtifactRepository::new();

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .list_artifacts(Parameters(ListArtifactsParams {
                issue_slug: "TP-1".to_string(),
                kinds: Some(vec!["NotAKind".to_string()]),
                uncovered_only: false,
            }))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_artifacts_non_member_returns_error() {
        let issue = fake_issue(10, 1, 1);
        let user = fake_user_with_caps(1, Capabilities::default());
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        let core = make_core_with_members(MockProjectRepository::new(), issue_repo, user_repo, member_repo);
        let result = IssueBossServer::new(core, user)
            .list_artifacts(Parameters(ListArtifactsParams {
                issue_slug: "TP-1".to_string(),
                kinds: None,
                uncovered_only: false,
            }))
            .await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // transition_issue — gate failure (tests GateFailure → map_core_err path)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn transition_issue_gate_failure() {
        use ib_core::artifact::MockArtifactRepository;
        // Issue is in TriageInProgress; transitioning to TriageReview requires a
        // TriageResult artifact. With no artifacts, the gate fails and
        // map_core_err must encode a gate_failed payload.
        let mut issue = fake_issue(10, 1, 1);
        issue.status = IssueStatus::TriageInProgress;

        let project_repo = MockProjectRepository::new();
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
        let mut artifact_repo = MockArtifactRepository::new();
        // Return empty list → no TriageResult → gate fails
        artifact_repo.expect_list().returning(|_, _, _| Box::pin(async { Ok(vec![]) }));

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .transition_issue(Parameters(TransitionIssueParams {
                slug: "TP-1".to_string(),
                new_status: "TriageReview".to_string(),
                reason: None,
            }))
            .await;

        assert!(result.is_err());
        // The error message from map_core_err contains the gate_failed JSON payload
        let err = result.unwrap_err();
        assert!(
            err.message.contains("gate_failed"),
            "expected gate_failed in error message, got: {}",
            err.message
        );
    }

    // -----------------------------------------------------------------------
    // move_artifact
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn move_artifact_happy_path() {
        use ib_core::artifact::{
            MockArtifactRepository,
            model::{ArtifactKind, ArtifactToken, IssueArtifact},
        };
        let issue = fake_issue(10, 1, 1);
        let artifact = IssueArtifact {
            id: 1,
            token: ArtifactToken::new(1),
            issue_id: 10,
            kind: ArtifactKind::Spec,
            slug: Some("spec".to_string()),
            body: serde_json::json!({"path": ".insights/new-spec.md"}),
            created_by: "U_1".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_id().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut artifact_repo = MockArtifactRepository::new();
        {
            let a = artifact.clone();
            artifact_repo.expect_find_by_path().returning(move |_, _| {
                let a = a.clone();
                Box::pin(async move { Ok(vec![a]) })
            });
        }
        artifact_repo.expect_update().returning(|_, a| Box::pin(async move { Ok(a) }));

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .move_artifact(Parameters(MoveArtifactParams {
                old_path: ".insights/old-spec.md".to_string(),
                new_path: ".insights/new-spec.md".to_string(),
            }))
            .await
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["updated"], 1);
        assert_eq!(parsed["artifacts"][0]["issue_slug"], "TP-1");
        assert_eq!(parsed["artifacts"][0]["slug"], "spec");
        assert_eq!(parsed["artifacts"][0]["kind"], "Spec");
    }

    #[tokio::test]
    async fn move_artifact_noop_same_path() {
        use ib_core::artifact::MockArtifactRepository;

        let project_repo = MockProjectRepository::new();
        let issue_repo = MockIssueRepository::new();
        // No expectations set — repository must NOT be called for same-path no-op
        let artifact_repo = MockArtifactRepository::new();

        let core = make_core_with_artifacts(project_repo, issue_repo, artifact_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .move_artifact(Parameters(MoveArtifactParams {
                old_path: ".insights/spec.md".to_string(),
                new_path: ".insights/spec.md".to_string(),
            }))
            .await
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["updated"], 0);
        assert!(parsed["artifacts"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn move_artifact_non_admin_returns_error() {
        // A non-admin user must not be able to move artifacts across projects.
        let user = fake_user_with_caps(1, Capabilities::default());
        let core = make_core(MockProjectRepository::new(), MockIssueRepository::new());
        let result = IssueBossServer::new(core, user)
            .move_artifact(Parameters(MoveArtifactParams {
                old_path: ".insights/old.md".to_string(),
                new_path: ".insights/new.md".to_string(),
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_issue_resource_non_member_returns_error() {
        let issue = fake_issue(10, 1, 1);
        let user = fake_user_with_caps(1, Capabilities::default());
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut user_repo = MockUserRepository::new();
        {
            let u = user.clone();
            user_repo.expect_find_by_id().returning(move |_, _| {
                let u = u.clone();
                Box::pin(async move { Ok(Some(u)) })
            });
        }
        let mut member_repo = MockProjectMemberRepository::new();
        member_repo.expect_find().returning(|_, _, _| Box::pin(async { Ok(None) }));
        let core = make_core_with_members(MockProjectRepository::new(), issue_repo, user_repo, member_repo);
        let result = IssueBossServer::new(core, user).read_resource_inner("issueboss://issues/TP-1").await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // add_relationship
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn add_relationship_happy_path() {
        use ib_core::relationship::{
            MockIssueRelationshipRepository,
            model::{IssueRelationship, IssueRelationships},
        };

        // Two issues in the same project
        let issue1 = fake_issue(1, 1, 1);
        let issue2 = fake_issue(2, 1, 2);

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            // The MCP handler calls find_by_slug for "TP-1" (capability check).
            // The service then calls find_by_slug for "TP-1" and "TP-2" inside
            // the transaction. We use a returning closure that matches on slug.
            let i1 = issue1.clone();
            let i2 = issue2.clone();
            issue_repo.expect_find_by_slug().returning(move |_, slug| {
                let result = if slug == "TP-1" { Some(i1.clone()) } else { Some(i2.clone()) };
                Box::pin(async move { Ok(result) })
            });
        }
        let mut rel_repo = MockIssueRelationshipRepository::new();
        // RelatedTo kind skips cycle check so list_for_issue is not called.
        rel_repo.expect_add().returning(|_, rec| {
            let kind = rec.kind.clone();
            let from = rec.from_issue_id;
            let to = rec.to_issue_id;
            Box::pin(async move {
                Ok(IssueRelationship {
                    id: 99,
                    from_issue_id: from,
                    to_issue_id: to,
                    kind,
                    created_at: chrono::Utc::now(),
                })
            })
        });
        // list_for_issue may be called during cycle check — return empty for safety.
        rel_repo
            .expect_list_for_issue()
            .returning(|_, _| Box::pin(async { Ok(IssueRelationships::default()) }));

        let core = make_core_with_relationships(project_repo, issue_repo, rel_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .add_relationship(Parameters(AddRelationshipParams {
                issue_slug: "TP-1".into(),
                related_slug: "TP-2".into(),
                kind: "RelatedTo".into(),
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["from"], "TP-1");
        assert_eq!(json["to"], "TP-2");
        assert_eq!(json["kind"], "RelatedTo");
    }

    #[tokio::test]
    async fn add_relationship_invalid_kind() {
        use ib_core::relationship::MockIssueRelationshipRepository;

        let project_repo = MockProjectRepository::new();
        let issue_repo = MockIssueRepository::new();
        let rel_repo = MockIssueRelationshipRepository::new();

        let core = make_core_with_relationships(project_repo, issue_repo, rel_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .add_relationship(Parameters(AddRelationshipParams {
                issue_slug: "TP-1".into(),
                related_slug: "TP-2".into(),
                kind: "InvalidKind".into(),
            }))
            .await;

        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // remove_relationship
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn remove_relationship_happy_path() {
        use ib_core::relationship::MockIssueRelationshipRepository;

        let issue1 = fake_issue(1, 1, 1);
        let issue2 = fake_issue(2, 1, 2);

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i1 = issue1.clone();
            let i2 = issue2.clone();
            issue_repo.expect_find_by_slug().returning(move |_, slug| {
                let result = if slug == "TP-1" { Some(i1.clone()) } else { Some(i2.clone()) };
                Box::pin(async move { Ok(result) })
            });
        }
        let mut rel_repo = MockIssueRelationshipRepository::new();
        rel_repo.expect_remove().returning(|_, _, _, _| Box::pin(async { Ok(true) }));

        let core = make_core_with_relationships(project_repo, issue_repo, rel_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .remove_relationship(Parameters(RemoveRelationshipParams {
                issue_slug: "TP-1".into(),
                related_slug: "TP-2".into(),
                kind: "DependsOn".into(),
            }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["ok"], true);
    }

    // -----------------------------------------------------------------------
    // list_relationships
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn list_relationships_happy_path() {
        use ib_core::relationship::{
            MockIssueRelationshipRepository,
            model::{IssueRelationships, RelatedIssueSummary},
        };

        let issue = fake_issue(10, 1, 1);

        let project_repo = MockProjectRepository::new();
        let mut issue_repo = MockIssueRepository::new();
        {
            let i = issue.clone();
            issue_repo.expect_find_by_slug().returning(move |_, _| {
                let i = i.clone();
                Box::pin(async move { Ok(Some(i)) })
            });
        }
        let mut rel_repo = MockIssueRelationshipRepository::new();
        rel_repo.expect_list_for_issue().returning(|_, _| {
            Box::pin(async {
                Ok(IssueRelationships {
                    depends_on: vec![RelatedIssueSummary {
                        id: 2,
                        slug: "TP-2".to_owned(),
                        title: "Issue 2".to_owned(),
                    }],
                    blocks: vec![],
                    related_to: vec![],
                })
            })
        });

        let core = make_core_with_relationships(project_repo, issue_repo, rel_repo);
        let server = IssueBossServer::new(core, fake_user(1));

        let result = server
            .list_relationships(Parameters(ListRelationshipsParams { issue_slug: "TP-1".into() }))
            .await;

        assert!(result.is_ok());
        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["depends_on"].as_array().unwrap().len(), 1);
        assert_eq!(json["depends_on"][0]["slug"], "TP-2");
        assert!(json["blocks"].as_array().unwrap().is_empty());
        assert!(json["related_to"].as_array().unwrap().is_empty());
    }

    #[test]
    fn artifact_mcp_includes_token() {
        use ib_core::artifact::{ArtifactKind, ArtifactToken, IssueArtifact};

        let token = ArtifactToken::new(42);
        let artifact = IssueArtifact {
            id: 1,
            token,
            issue_id: 10,
            kind: ArtifactKind::Comment,
            slug: Some("my-comment".into()),
            body: serde_json::json!({"text": "hello"}),
            created_by: "U_test".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mcp = ArtifactMcp::from_artifact(artifact);
        let json = serde_json::to_value(&mcp).unwrap();

        assert_eq!(json["token"], token.to_string());
    }

    #[tokio::test]
    async fn list_issues_exclude_blocked_param_accepted() {
        let json = r#"{"project_slug": "ib", "exclude_blocked": true}"#;
        let params: ListIssuesParams = serde_json::from_str(json).expect("should deserialize");
        assert_eq!(params.exclude_blocked, Some(true));

        let json_false = r#"{"project_slug": "ib", "exclude_blocked": false}"#;
        let params_false: ListIssuesParams = serde_json::from_str(json_false).expect("should deserialize");
        assert_eq!(params_false.exclude_blocked, Some(false));

        let json_omit = r#"{"project_slug": "ib"}"#;
        let params_omit: ListIssuesParams = serde_json::from_str(json_omit).expect("should deserialize");
        assert_eq!(params_omit.exclude_blocked, None);
    }
}
