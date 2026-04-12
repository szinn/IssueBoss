use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub http_port: u16,
    pub mcp_port: u16,
    pub grpc_port: u16,
    pub database_url: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let cfg = config::Config::builder()
            .set_default("http_port", 8080)?
            .set_default("mcp_port", 8090)?
            .set_default("grpc_port", 9090)?
            .add_source(config::Environment::with_prefix("ISSUEBOSS").separator("__"))
            .build()?;
        Ok(cfg.try_deserialize()?)
    }
}
