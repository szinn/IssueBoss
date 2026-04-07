use ib_api::grpc::{
    admin::project,
    admin_proto::{ProjectMemberResponse, ProjectResponse},
};

// ── Args ─────────────────────────────────────────────────────────────────────

#[derive(Debug, clap::Args)]
pub(crate) struct ProjectArgs {
    #[clap(subcommand)]
    pub command: ProjectCommands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum ProjectCommands {
    #[command(about = "Create a project")]
    Create(CreateArgs),
    #[command(about = "List all projects")]
    List,
    #[command(about = "Get a project by slug")]
    Get(GetArgs),
    #[command(about = "Update a project")]
    Update(UpdateArgs),
    #[command(about = "Delete a project")]
    Delete(DeleteArgs),
    #[command(about = "Add a member to a project", name = "add-member")]
    AddMember(AddMemberArgs),
    #[command(about = "Update a project member's capabilities", name = "update-member")]
    UpdateMember(UpdateMemberArgs),
    #[command(about = "Remove a member from a project", name = "remove-member")]
    RemoveMember(RemoveMemberArgs),
    #[command(about = "List all members of a project", name = "list-members")]
    ListMembers(ListMembersArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct CreateArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub slug: String,
    #[arg(long)]
    pub prefix: String,
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct GetArgs {
    #[arg(long)]
    pub project: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct UpdateArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct DeleteArgs {
    #[arg(long)]
    pub project: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct AddMemberArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub username: String,
    /// Capabilities (may be specified multiple times)
    #[arg(long)]
    pub capability: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct UpdateMemberArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub username: String,
    /// Capabilities (may be specified multiple times)
    #[arg(long)]
    pub capability: Vec<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RemoveMemberArgs {
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub username: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ListMembersArgs {
    #[arg(long)]
    pub project: String,
}

// ── Dispatcher ───────────────────────────────────────────────────────────────

pub(crate) async fn cmd_project(host: &str, port: u16, args: ProjectArgs) -> anyhow::Result<()> {
    match args.command {
        ProjectCommands::Create(a) => cmd_create(host, port, a).await,
        ProjectCommands::List => cmd_list(host, port).await,
        ProjectCommands::Get(a) => cmd_get(host, port, a).await,
        ProjectCommands::Update(a) => cmd_update(host, port, a).await,
        ProjectCommands::Delete(a) => cmd_delete(host, port, a).await,
        ProjectCommands::AddMember(a) => cmd_add_member(host, port, a).await,
        ProjectCommands::UpdateMember(a) => cmd_update_member(host, port, a).await,
        ProjectCommands::RemoveMember(a) => cmd_remove_member(host, port, a).await,
        ProjectCommands::ListMembers(a) => cmd_list_members(host, port, a).await,
    }
}

// ── Implementations ──────────────────────────────────────────────────────────

async fn cmd_create(host: &str, port: u16, args: CreateArgs) -> anyhow::Result<()> {
    let p = project::api::create_project(host, port, &args.name, &args.slug, &args.prefix, args.description.as_deref())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_project(&p);
    Ok(())
}

async fn cmd_list(host: &str, port: u16) -> anyhow::Result<()> {
    let resp = project::api::list_projects(host, port).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    if resp.projects.is_empty() {
        println!("No projects.");
        return Ok(());
    }
    println!("{:<15} {:<25} {:<8} TOKEN", "SLUG", "NAME", "PREFIX");
    println!("{}", "-".repeat(80));
    for p in resp.projects {
        println!("{:<15} {:<25} {:<8} {}", p.slug, p.name, p.prefix, p.token);
    }
    Ok(())
}

async fn cmd_get(host: &str, port: u16, args: GetArgs) -> anyhow::Result<()> {
    let p = project::api::get_project(host, port, &args.project).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    print_project(&p);
    Ok(())
}

async fn cmd_update(host: &str, port: u16, args: UpdateArgs) -> anyhow::Result<()> {
    if args.name.is_none() {
        anyhow::bail!("--name must be provided");
    }
    let p = project::api::update_project(host, port, &args.project, args.name.as_deref())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_project(&p);
    Ok(())
}

async fn cmd_delete(host: &str, port: u16, args: DeleteArgs) -> anyhow::Result<()> {
    project::api::delete_project(host, port, &args.project)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Deleted project: {}", args.project);
    Ok(())
}

async fn cmd_add_member(host: &str, port: u16, args: AddMemberArgs) -> anyhow::Result<()> {
    let m = project::api::add_project_member(host, port, &args.project, &args.username, args.capability)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_member(&m);
    Ok(())
}

async fn cmd_update_member(host: &str, port: u16, args: UpdateMemberArgs) -> anyhow::Result<()> {
    let m = project::api::update_project_member(host, port, &args.project, &args.username, args.capability)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_member(&m);
    Ok(())
}

async fn cmd_remove_member(host: &str, port: u16, args: RemoveMemberArgs) -> anyhow::Result<()> {
    project::api::remove_project_member(host, port, &args.project, &args.username)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Removed {} from project {}", args.username, args.project);
    Ok(())
}

async fn cmd_list_members(host: &str, port: u16, args: ListMembersArgs) -> anyhow::Result<()> {
    let resp = project::api::list_project_members(host, port, &args.project)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if resp.members.is_empty() {
        println!("No members.");
        return Ok(());
    }
    println!("{:<20} CAPABILITIES", "USERNAME");
    println!("{}", "-".repeat(50));
    for m in resp.members {
        println!("{:<20} {}", m.username, m.capabilities.join(", "));
    }
    Ok(())
}

// ── Output helpers
// ────────────────────────────────────────────────────────────

fn print_project(p: &ProjectResponse) {
    println!("Token:    {}", p.token);
    println!("Name:     {}", p.name);
    println!("Slug:     {}", p.slug);
    println!("Prefix:   {}", p.prefix);
    if let Some(desc) = &p.description {
        println!("Desc:     {desc}");
    }
    println!("Created:  {}", p.created_at);
    println!("Updated:  {}", p.updated_at);
}

fn print_member(m: &ProjectMemberResponse) {
    println!("Project:  {}", m.project_token);
    println!("User:     {}", m.username);
    println!("Token:    {}", m.user_token);
    println!("Caps:     {}", m.capabilities.join(", "));
    println!("Created:  {}", m.created_at);
    println!("Updated:  {}", m.updated_at);
}
