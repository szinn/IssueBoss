mod commands;
mod config;
mod tracing;

use clap::Parser;
use commands::{CommandLine, Commands};

fn main() -> anyhow::Result<()> {
    let cli = CommandLine::parse();
    crate::tracing::init_tracing(cli.verbose);
    match cli.command {
        Commands::Init(args) => commands::init::cmd_init(args, cli.verbose),
        Commands::Sync => commands::sync::cmd_sync(cli.verbose),
        Commands::Commit => commands::commit::cmd_commit(cli.verbose),
    }
}
