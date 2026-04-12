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

    match cli.command {
        Commands::Server => {
            let config = Config::load()?;
            init_logging()?;
            cmd_server(config).await
        }
        Commands::SuperAdmin(args) => commands::super_admin::cmd_super_admin(&cli.host, cli.port, args).await,
        Commands::User(args) => {
            use crate::commands::user::cmd_user;
            cmd_user(&cli.host, cli.port, args).await
        }
        Commands::Project(args) => {
            use crate::commands::project::cmd_project;
            cmd_project(&cli.host, cli.port, args).await
        }
        Commands::Issue(args) => {
            use crate::commands::issue::cmd_issue;
            cmd_issue(&cli.host, cli.port, args).await
        }
        Commands::Artifact(args) => {
            use crate::commands::artifact::cmd_artifact;
            cmd_artifact(&cli.host, cli.port, args).await
        }
        Commands::Relationship(args) => {
            use crate::commands::relationship::cmd_relationship;
            cmd_relationship(&cli.host, cli.port, args).await
        }
        Commands::ApiKey(args) => {
            use crate::commands::api_key::cmd_api_key;
            cmd_api_key(&cli.host, cli.port, args).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::config::Config;

    #[test]
    fn server_config_requires_database_url() {
        // Config::load() must fail without database_url — confirming it is
        // server-only config that must not be called for gRPC client commands.
        // SAFETY: single-threaded test process; no other threads read this var.
        unsafe { std::env::remove_var("ISSUEBOSS__DATABASE_URL") };
        assert!(Config::load().is_err());
    }
}
