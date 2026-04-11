use crate::{config::Config, core::commit::commit};

pub fn cmd_commit(verbose: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    commit(&config, verbose)
}
