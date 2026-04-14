use ib_api::grpc::admin::issue;

use crate::display;

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
    #[command(about = "Transition an issue to a new status")]
    Transition(TransitionArgs),
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
    #[arg(long)]
    pub exclude_blocked: Option<bool>,
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

#[derive(Debug, clap::Args)]
pub(crate) struct TransitionArgs {
    /// Issue slug (e.g. IB-1)
    #[arg(long)]
    pub issue: String,
    /// New status (e.g. SpecNeeded, InDev, Done)
    #[arg(long)]
    pub status: String,
}

// ── Dispatcher ───────────────────────────────────────────────────────────────

pub(crate) async fn cmd_issue(host: &str, port: u16, args: IssueArgs) -> anyhow::Result<()> {
    match args.command {
        IssueCommands::Create(a) => cmd_create(host, port, a).await,
        IssueCommands::List(a) => cmd_list(host, port, a).await,
        IssueCommands::Get(a) => cmd_get(host, port, a).await,
        IssueCommands::Update(a) => cmd_update(host, port, a).await,
        IssueCommands::Transition(a) => cmd_transition(host, port, a).await,
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
    print!("{}", display::format_issue(&i));
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
        args.exclude_blocked,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    print!("{}", display::format_issue_list(&resp.issues));
    Ok(())
}

async fn cmd_get(host: &str, port: u16, args: GetArgs) -> anyhow::Result<()> {
    let i = issue::api::get_issue(host, port, &args.issue).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    print!("{}", display::format_issue(&i));
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
        args.priority.as_deref(),
        args.size.as_deref(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    print!("{}", display::format_issue(&i));
    Ok(())
}

async fn cmd_transition(host: &str, port: u16, args: TransitionArgs) -> anyhow::Result<()> {
    let i = issue::api::transition_issue(host, port, &args.issue, &args.status)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print!("{}", display::format_issue(&i));
    Ok(())
}
