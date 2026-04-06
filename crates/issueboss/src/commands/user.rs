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
    #[command(about = "Rotate a user's API key", name = "rotate-api-key")]
    RotateApiKey(RotateApiKeyArgs),
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
pub(crate) struct RotateApiKeyArgs {
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
        UserCommands::RotateApiKey(a) => cmd_rotate_api_key(host, port, a).await,
    }
}

// ── Implementations ──────────────────────────────────────────────────────────

async fn cmd_create(host: &str, port: u16, args: CreateArgs) -> anyhow::Result<()> {
    let user = user::api::create_user(host, port, &args.username, &args.full_name, &args.email, &args.password)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_user(&user);
    Ok(())
}

async fn cmd_list(host: &str, port: u16) -> anyhow::Result<()> {
    let users = user::api::list_users(host, port).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    if users.is_empty() {
        println!("No users.");
        return Ok(());
    }
    println!("{:<20} {:<25} {:<30} {:<15} API KEY PREFIX", "USERNAME", "FULL NAME", "EMAIL", "CAPABILITIES");
    println!("{}", "-".repeat(110));
    for u in users {
        let caps = u.capabilities.join(", ");
        let prefix = if u.api_key_prefix.is_empty() { "-".to_owned() } else { u.api_key_prefix };
        println!("{:<20} {:<25} {:<30} {:<15} {}", u.username, u.full_name, u.email, caps, prefix);
    }
    Ok(())
}

async fn cmd_get(host: &str, port: u16, args: GetArgs) -> anyhow::Result<()> {
    let user = user::api::get_user(host, port, &args.username).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    print_user(&user);
    Ok(())
}

async fn cmd_update(host: &str, port: u16, args: UpdateArgs) -> anyhow::Result<()> {
    if args.full_name.is_none() && args.email.is_none() {
        anyhow::bail!("At least one of --full-name or --email must be provided");
    }
    let user = user::api::update_user(host, port, &args.username, args.full_name.as_deref(), args.email.as_deref())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    print_user(&user);
    Ok(())
}

async fn cmd_delete(host: &str, port: u16, args: DeleteArgs) -> anyhow::Result<()> {
    user::api::delete_user(host, port, &args.username).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Deleted user: {}", args.username);
    Ok(())
}

async fn cmd_rotate_api_key(host: &str, port: u16, args: RotateApiKeyArgs) -> anyhow::Result<()> {
    let resp = user::api::rotate_api_key(host, port, &args.username)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("\nAPI key rotated for: {}", resp.username);
    println!("  API key: {}", resp.api_key);
    println!("  Prefix:  {}", resp.api_key_prefix);
    println!("\nStore this key securely — it will not be shown again.");
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
    println!("Key prefix: {}", if u.api_key_prefix.is_empty() { "(none)" } else { &u.api_key_prefix });
    println!("Created:    {}", u.created_at);
    println!("Updated:    {}", u.updated_at);
}
