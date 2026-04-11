use crate::{config::Config, core::sync::sync};

pub fn cmd_sync(verbose: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    sync(&config, verbose)
}
