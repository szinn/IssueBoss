use ib_api::grpc::admin::artifact;

use crate::display;

// ── Args ─────────────────────────────────────────────────────────────────────

#[derive(Debug, clap::Args)]
pub(crate) struct ArtifactArgs {
    #[clap(subcommand)]
    pub command: ArtifactCommands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum ArtifactCommands {
    #[command(about = "Add an artifact to an issue")]
    Add(AddArgs),
    #[command(about = "Update an artifact")]
    Update(UpdateArgs),
    #[command(about = "Remove an artifact from an issue")]
    Remove(RemoveArgs),
    #[command(about = "List artifacts for an issue")]
    List(ListArgs),
    #[command(about = "Move an artifact to a new path")]
    Move(MoveArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct AddArgs {
    /// Issue slug (e.g. IB-7)
    #[arg(long)]
    pub issue: String,
    /// Artifact kind (e.g. Comment, Research, Spec)
    #[arg(long)]
    pub kind: String,
    /// Artifact slug (required for caller-slug kinds: Comment, Research,
    /// Handoff, ResearchTopic)
    #[arg(long)]
    pub slug: Option<String>,
    /// Artifact body as JSON string (e.g. '{"text":"..."}')
    #[arg(long)]
    pub body: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct UpdateArgs {
    /// Issue slug (e.g. IB-7)
    #[arg(long)]
    pub issue: String,
    /// Artifact number
    #[arg(long)]
    pub artifact_number: u32,
    /// Artifact body as JSON string
    #[arg(long)]
    pub body: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RemoveArgs {
    /// Issue slug (e.g. IB-7)
    #[arg(long)]
    pub issue: String,
    /// Artifact number
    #[arg(long)]
    pub artifact_number: u32,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ListArgs {
    /// Issue slug (e.g. IB-7)
    #[arg(long)]
    pub issue: String,
    /// Comma-separated list of kinds to filter (e.g. "Comment,Research")
    #[arg(long)]
    pub kinds: Option<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct MoveArgs {
    /// Old artifact path
    #[arg(long)]
    pub old_path: String,
    /// New artifact path
    #[arg(long)]
    pub new_path: String,
}

// ── Dispatcher ───────────────────────────────────────────────────────────────

pub(crate) async fn cmd_artifact(host: &str, port: u16, args: ArtifactArgs) -> anyhow::Result<()> {
    match args.command {
        ArtifactCommands::Add(a) => cmd_add(host, port, a).await,
        ArtifactCommands::Update(a) => cmd_update(host, port, a).await,
        ArtifactCommands::Remove(a) => cmd_remove(host, port, a).await,
        ArtifactCommands::List(a) => cmd_list(host, port, a).await,
        ArtifactCommands::Move(a) => cmd_move(host, port, a).await,
    }
}

// ── Implementations ──────────────────────────────────────────────────────────

const CALLER_SLUG_KINDS: &[&str] = &["Comment", "Research", "Handoff", "ResearchTopic"];

async fn cmd_add(host: &str, port: u16, args: AddArgs) -> anyhow::Result<()> {
    let is_caller_slug_kind = CALLER_SLUG_KINDS.iter().any(|k| k.eq_ignore_ascii_case(&args.kind));
    if is_caller_slug_kind && args.slug.is_none() {
        anyhow::bail!("--slug is required for artifact kind '{}'", args.kind);
    }
    let a = artifact::api::add_artifact(host, port, &args.issue, &args.kind, args.slug.as_deref(), &args.body)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print!("{}", display::format_artifact(&a));
    Ok(())
}

async fn cmd_update(host: &str, port: u16, args: UpdateArgs) -> anyhow::Result<()> {
    let a = artifact::api::update_artifact(host, port, &args.issue, args.artifact_number, &args.body)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print!("{}", display::format_artifact(&a));
    Ok(())
}

async fn cmd_remove(host: &str, port: u16, args: RemoveArgs) -> anyhow::Result<()> {
    artifact::api::remove_artifact(host, port, &args.issue, args.artifact_number)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Artifact removed.");
    Ok(())
}

async fn cmd_list(host: &str, port: u16, args: ListArgs) -> anyhow::Result<()> {
    let kinds: Vec<String> = args
        .kinds
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let resp = artifact::api::list_artifacts(host, port, &args.issue, kinds)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print!("{}", display::format_artifact_list(&resp.artifacts));
    Ok(())
}

async fn cmd_move(host: &str, port: u16, args: MoveArgs) -> anyhow::Result<()> {
    artifact::api::move_artifact(host, port, &args.old_path, &args.new_path)
        .await
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Artifact moved.");
    Ok(())
}
