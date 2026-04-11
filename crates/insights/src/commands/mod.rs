pub mod commit;
pub mod init;
pub mod status;
pub mod sync;

#[derive(Debug, clap::Parser)]
#[command(
    name = "insights",
    about = "Manage the Insights knowledge repository connection",
    version,
    arg_required_else_help = true,
    propagate_version = true
)]
pub struct CommandLine {
    /// Enable verbose output (logs commands, symlink operations, etc.)
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    #[command(about = "Connect this project to an Insights repo")]
    Init(init::InitArgs),

    #[command(about = "Pull latest from the Insights repo and rebuild local state")]
    Sync,

    #[command(about = "Sync and show pending changes in the Insights repo")]
    Status,

    #[command(about = "Sync and commit all pending changes to the Insights repo")]
    Commit,
}
