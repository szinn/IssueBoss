use ib_api::grpc::{admin::user, admin_proto::UserResponse};

use crate::display;

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

// ── Dispatcher ───────────────────────────────────────────────────────────────

pub(crate) async fn cmd_user(host: &str, port: u16, args: UserArgs) -> anyhow::Result<()> {
    match args.command {
        UserCommands::Create(a) => cmd_create(host, port, a).await,
        UserCommands::List => cmd_list(host, port).await,
        UserCommands::Get(a) => cmd_get(host, port, a).await,
        UserCommands::Update(a) => cmd_update(host, port, a).await,
        UserCommands::Delete(a) => cmd_delete(host, port, a).await,
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
    print!("{}", display::format_user_list(&users));
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

// ── Output helper
// ─────────────────────────────────────────────────────────────

fn print_user(u: &UserResponse) {
    print!("{}", display::format_user(u));
}
