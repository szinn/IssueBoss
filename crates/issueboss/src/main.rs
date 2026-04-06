#[cfg(feature = "server")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod commands;
mod config;
mod logging;

#[cfg(feature = "server")]
use crate::{
    commands::{CommandLine, Commands, server::cmd_server},
    config::Config,
    logging::init_logging,
};

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: CommandLine = clap::Parser::parse();
    let config = Config::load()?;

    match cli.command {
        Commands::Server => {
            init_logging()?;
            cmd_server(config).await
        }
        Commands::SuperAdmin(args) => commands::super_admin::cmd_super_admin(&cli.host, cli.port, args).await,
        Commands::User(args) => {
            use crate::commands::user::cmd_user;
            cmd_user(&cli.host, cli.port, args).await
        }
    }
}
