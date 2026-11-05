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
    // Canonicalize so the stored config and symlink targets are clean absolute
    // paths, even if --repo was given as relative or with repeated slashes.
    let repo = args
        .repo
        .canonicalize()
        .with_context(|| format!("Insights repo '{}' not found", args.repo.display()))?;
    init(
        InitOptions {
            repo,
            user: args.user,
            project: args.project,
            project_root,
        },
        verbose,
    )?;
    println!("Initialized.");
    Ok(())
}
