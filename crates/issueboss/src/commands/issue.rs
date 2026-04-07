use ib_api::grpc::{admin::issue, admin_proto::IssueResponse};

// ── Args ─────────────────────────────────────────────────────────────────────

#[derive(Debug, clap::Args)]
pub(crate) struct IssueArgs {
    #[clap(subcommand)]
    pub command: IssueCommands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum IssueCommands {
    #[command(about = "Create an issue")]
    Create(CreateArgs),
    #[command(about = "List issues in a project")]
    List(ListArgs),
    #[command(about = "Get an issue by token")]
    Get(GetArgs),
    #[command(about = "Update an issue")]
    Update(UpdateArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct CreateArgs {
    /// Project slug
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub title: String,
    #[arg(long)]
    pub description: Option<String>,
    /// Priority: Urgent, High, Medium (default), Low
    #[arg(long)]
    pub priority: Option<String>,
    /// Size: XS, Small, Medium, Large
    #[arg(long)]
    pub size: Option<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ListArgs {
    /// Project slug
    #[arg(long)]
    pub project: String,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub priority: Option<String>,
    #[arg(long)]
    pub size: Option<String>,
    #[arg(long)]
    pub limit: Option<u64>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct GetArgs {
    /// Issue slug (e.g. IB-1)
    #[arg(long)]
    pub issue: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct UpdateArgs {
    /// Issue slug (e.g. IB-1)
    #[arg(long)]
    pub issue: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub priority: Option<String>,
    #[arg(long)]
    pub size: Option<String>,
}

// ── Dispatcher ───────────────────────────────────────────────────────────────

pub(crate) async fn cmd_issue(host: &str, port: u16, args: IssueArgs) -> anyhow::Result<()> {
    match args.command {
        IssueCommands::Create(a) => cmd_create(host, port, a).await,
        IssueCommands::List(a) => cmd_list(host, port, a).await,
        IssueCommands::Get(a) => cmd_get(host, port, a).await,
        IssueCommands::Update(a) => cmd_update(host, port, a).await,
    }
}

// ── Implementations ──────────────────────────────────────────────────────────

async fn cmd_create(host: &str, port: u16, args: CreateArgs) -> anyhow::Result<()> {
    let i = issue::api::create_issue(
        host,
        port,
        &args.project,
        &args.title,
        args.description.as_deref().unwrap_or(""),
        args.priority.as_deref().unwrap_or("Medium"),
        args.size.as_deref(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_issue(&i);
    Ok(())
}

async fn cmd_list(host: &str, port: u16, args: ListArgs) -> anyhow::Result<()> {
    let resp = issue::api::list_issues(
        host,
        port,
        &args.project,
        args.status.as_deref(),
        args.priority.as_deref(),
        args.size.as_deref(),
        args.limit,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    if resp.issues.is_empty() {
        println!("No issues.");
        return Ok(());
    }
    println!("{:<10} {:<12} {:<10} {:<8} TITLE", "REF", "STATUS", "PRIORITY", "SIZE");
    println!("{}", "-".repeat(80));
    for i in resp.issues {
        println!(
            "{:<10} {:<12} {:<10} {:<8} {}",
            i.slug,
            i.status,
            i.priority,
            i.size.as_deref().unwrap_or("-"),
            i.title,
        );
    }
    Ok(())
}

async fn cmd_get(host: &str, port: u16, args: GetArgs) -> anyhow::Result<()> {
    let i = issue::api::get_issue(host, port, &args.issue).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    print_issue(&i);
    Ok(())
}

async fn cmd_update(host: &str, port: u16, args: UpdateArgs) -> anyhow::Result<()> {
    if args.title.is_none() && args.description.is_none() && args.priority.is_none() && args.size.is_none() {
        anyhow::bail!("at least one of --title, --description, --priority, --size must be provided");
    }
    let i = issue::api::update_issue(
        host,
        port,
        &args.issue,
        args.title.as_deref(),
        args.description.as_deref(),
        None, // status transitions deferred to M3c
        args.priority.as_deref(),
        args.size.as_deref(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_issue(&i);
    Ok(())
}

// ── Output helpers
// ────────────────────────────────────────────────────────────

fn print_issue(i: &IssueResponse) {
    println!("Reference:   {}", i.slug);
    println!("Token:       {}", i.token);
    println!("Title:       {}", i.title);
    println!("Status:      {}", i.status);
    println!("Priority:    {}", i.priority);
    println!("Size:        {}", i.size.as_deref().unwrap_or("-"));
    if !i.description.is_empty() {
        println!("Description: {}", i.description);
    }
    println!("Created:     {}", i.created_at);
    println!("Updated:     {}", i.updated_at);
}
