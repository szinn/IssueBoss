use ib_api::grpc::admin::api_key;

// ── Args ─────────────────────────────────────────────────────────────────────

#[derive(Debug, clap::Args)]
pub(crate) struct ApiKeyArgs {
    #[clap(subcommand)]
    pub command: ApiKeyCommands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum ApiKeyCommands {
    #[command(about = "Create an API key for a user")]
    Create(CreateArgs),
    #[command(about = "Revoke an API key by its numeric ID")]
    Revoke(RevokeArgs),
    #[command(about = "List all API keys for a user")]
    List(ListArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct CreateArgs {
    #[arg(long)]
    pub username: String,
    #[arg(long, default_value = "ib_live")]
    pub key_type: String,
    #[arg(long, default_value = "")]
    pub name: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RevokeArgs {
    /// The numeric ID returned when the key was created.
    #[arg(long)]
    pub api_key_id: u64,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ListArgs {
    #[arg(long)]
    pub username: String,
}

// ── Dispatcher ───────────────────────────────────────────────────────────────

pub(crate) async fn cmd_api_key(host: &str, port: u16, args: ApiKeyArgs) -> anyhow::Result<()> {
    match args.command {
        ApiKeyCommands::Create(a) => cmd_create(host, port, a).await,
        ApiKeyCommands::Revoke(a) => cmd_revoke(host, port, a).await,
        ApiKeyCommands::List(a) => cmd_list(host, port, a).await,
    }
}

// ── Implementations ──────────────────────────────────────────────────────────

async fn cmd_create(host: &str, port: u16, args: CreateArgs) -> anyhow::Result<()> {
    let resp = api_key::api::create_api_key(host, port, &args.username, &args.key_type, &args.name)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("\nAPI key created for: {}", resp.username);
    println!("  Type:    {}", resp.key_type);
    println!("  Prefix:  {}", resp.key_prefix);
    println!("  ID:      {}", resp.api_key_id);
    println!("  API key: {}", resp.api_key);
    println!("\nStore this key securely — it will not be shown again.");
    Ok(())
}

async fn cmd_revoke(host: &str, port: u16, args: RevokeArgs) -> anyhow::Result<()> {
    api_key::api::revoke_api_key(host, port, args.api_key_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Revoked API key: {}", args.api_key_id);
    Ok(())
}

async fn cmd_list(host: &str, port: u16, args: ListArgs) -> anyhow::Result<()> {
    let resp = api_key::api::list_api_keys(host, port, &args.username)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if resp.keys.is_empty() {
        println!("No API keys for {}.", args.username);
        return Ok(());
    }
    println!("{:<20} {:<12} {:<20} {:<25} LAST USED", "ID", "TYPE", "PREFIX", "NAME");
    println!("{}", "-".repeat(100));
    for k in resp.keys {
        let last = if k.last_used_at.is_empty() { "-".to_owned() } else { k.last_used_at };
        println!("{:<20} {:<12} {:<20} {:<25} {}", k.api_key_id, k.key_type, k.key_prefix, k.name, last);
    }
    Ok(())
}
