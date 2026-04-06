use ib_api::grpc::{admin::user, admin_proto::UserResponse};

// ── Args ─────────────────────────────────────────────────────────────────────

#[derive(Debug, clap::Args)]
pub(crate) struct UserArgs {
    #[clap(subcommand)]
    pub command: UserCommands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum UserCommands {
    #[command(about = "Create a user")]
    Create(CreateArgs),
    #[command(about = "List all users")]
    List,
    #[command(about = "Get a user by username")]
    Get(GetArgs),
    #[command(about = "Update a user's full name or email")]
    Update(UpdateArgs),
    #[command(about = "Delete a user")]
    Delete(DeleteArgs),
    #[command(about = "Create an API key for a user", name = "create-api-key")]
    CreateApiKey(CreateApiKeyArgs),
    #[command(about = "Revoke an API key by its token", name = "revoke-api-key")]
    RevokeApiKey(RevokeApiKeyArgs),
    #[command(about = "List all API keys for a user", name = "list-api-keys")]
    ListApiKeys(ListApiKeysArgs),
}

#[derive(Debug, clap::Args)]
pub(crate) struct CreateArgs {
    #[arg(long)]
    pub username: String,
    #[arg(long)]
    pub full_name: String,
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub password: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct GetArgs {
    #[arg(long)]
    pub username: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct UpdateArgs {
    #[arg(long)]
    pub username: String,
    #[arg(long)]
    pub full_name: Option<String>,
    #[arg(long)]
    pub email: Option<String>,
}

#[derive(Debug, clap::Args)]
pub(crate) struct DeleteArgs {
    #[arg(long)]
    pub username: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct CreateApiKeyArgs {
    #[arg(long)]
    pub username: String,
    #[arg(long, default_value = "ib_live")]
    pub key_type: String,
    #[arg(long, default_value = "")]
    pub name: String,
}

#[derive(Debug, clap::Args)]
pub(crate) struct RevokeApiKeyArgs {
    /// The numeric ID returned when the key was created.
    #[arg(long)]
    pub api_key_id: u64,
}

#[derive(Debug, clap::Args)]
pub(crate) struct ListApiKeysArgs {
    #[arg(long)]
    pub username: String,
}

// ── Dispatcher ───────────────────────────────────────────────────────────────

pub(crate) async fn cmd_user(host: &str, port: u16, args: UserArgs) -> anyhow::Result<()> {
    match args.command {
        UserCommands::Create(a) => cmd_create(host, port, a).await,
        UserCommands::List => cmd_list(host, port).await,
        UserCommands::Get(a) => cmd_get(host, port, a).await,
        UserCommands::Update(a) => cmd_update(host, port, a).await,
        UserCommands::Delete(a) => cmd_delete(host, port, a).await,
        UserCommands::CreateApiKey(a) => cmd_create_api_key(host, port, a).await,
        UserCommands::RevokeApiKey(a) => cmd_revoke_api_key(host, port, a).await,
        UserCommands::ListApiKeys(a) => cmd_list_api_keys(host, port, a).await,
    }
}

// ── Implementations ──────────────────────────────────────────────────────────

async fn cmd_create(host: &str, port: u16, args: CreateArgs) -> anyhow::Result<()> {
    let u = user::api::create_user(host, port, &args.username, &args.full_name, &args.email, &args.password)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_user(&u);
    Ok(())
}

async fn cmd_list(host: &str, port: u16) -> anyhow::Result<()> {
    let users = user::api::list_users(host, port).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    if users.is_empty() {
        println!("No users.");
        return Ok(());
    }
    println!("{:<20} {:<25} {:<30} CAPABILITIES", "USERNAME", "FULL NAME", "EMAIL");
    println!("{}", "-".repeat(90));
    for u in users {
        println!("{:<20} {:<25} {:<30} {}", u.username, u.full_name, u.email, u.capabilities.join(", "));
    }
    Ok(())
}

async fn cmd_get(host: &str, port: u16, args: GetArgs) -> anyhow::Result<()> {
    let u = user::api::get_user(host, port, &args.username).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    print_user(&u);
    Ok(())
}

async fn cmd_update(host: &str, port: u16, args: UpdateArgs) -> anyhow::Result<()> {
    if args.full_name.is_none() && args.email.is_none() {
        anyhow::bail!("At least one of --full-name or --email must be provided");
    }
    let u = user::api::update_user(host, port, &args.username, args.full_name.as_deref(), args.email.as_deref())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_user(&u);
    Ok(())
}

async fn cmd_delete(host: &str, port: u16, args: DeleteArgs) -> anyhow::Result<()> {
    user::api::delete_user(host, port, &args.username).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Deleted user: {}", args.username);
    Ok(())
}

async fn cmd_create_api_key(host: &str, port: u16, args: CreateApiKeyArgs) -> anyhow::Result<()> {
    let resp = user::api::create_api_key(host, port, &args.username, &args.key_type, &args.name)
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

async fn cmd_revoke_api_key(host: &str, port: u16, args: RevokeApiKeyArgs) -> anyhow::Result<()> {
    user::api::revoke_api_key(host, port, args.api_key_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Revoked API key: {}", args.api_key_id);
    Ok(())
}

async fn cmd_list_api_keys(host: &str, port: u16, args: ListApiKeysArgs) -> anyhow::Result<()> {
    let resp = user::api::list_api_keys(host, port, &args.username).await.map_err(|e| anyhow::anyhow!("{e}"))?;
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

// ── Output helper
// ─────────────────────────────────────────────────────────────

fn print_user(u: &UserResponse) {
    println!("Username:   {}", u.username);
    println!("Full name:  {}", u.full_name);
    println!("Email:      {}", u.email);
    println!("Token:      {}", u.token);
    println!("Caps:       {}", u.capabilities.join(", "));
    println!("Created:    {}", u.created_at);
    println!("Updated:    {}", u.updated_at);
}
