#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// Absolute path to the local Insights git repo clone
    #[arg(long)]
    pub repo: std::path::PathBuf,

    /// Your username (maps to users/<user>/ in the repo)
    #[arg(long)]
    pub user: String,

    /// Project name (e.g. "IssueBoss"; lowercased to derive repo directory)
    #[arg(long)]
    pub project: String,
}

pub fn cmd_init(_args: InitArgs, _verbose: bool) -> anyhow::Result<()> {
    Ok(())
}
