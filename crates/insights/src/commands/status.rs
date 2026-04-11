use crate::{config::Config, core::status::status};

pub fn cmd_status(verbose: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    status(&config, verbose)
}
