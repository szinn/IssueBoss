use std::path::PathBuf;

use anyhow::Context;

use crate::core::init::{InitOptions, init};

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    /// Absolute path to the local Insights git repo clone
    #[arg(long)]
    pub repo: PathBuf,

    /// Your username (maps to users/<user>/ in the repo)
    #[arg(long)]
    pub user: String,

    /// Project name (e.g. "IssueBoss"; lowercased to derive repo directory)
    #[arg(long)]
    pub project: String,
}

pub fn cmd_init(args: InitArgs, verbose: bool) -> anyhow::Result<()> {
    let project_root = std::env::current_dir().context("Failed to determine current directory")?;
    init(
        InitOptions {
            repo: args.repo,
            user: args.user,
            project: args.project,
            project_root,
        },
        verbose,
    )
}
