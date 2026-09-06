#[cfg(feature = "server")]
use anyhow::Result;
#[cfg(feature = "server")]
use serde::Deserialize;

#[cfg(feature = "server")]
#[derive(Debug, Deserialize)]
pub struct Config {
    pub http_port: u16,
    pub mcp_port: u16,
    pub grpc_port: u16,
    pub database_url: String,
}

#[cfg(feature = "server")]
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

#[cfg(all(test, feature = "server"))]
mod tests {
    use temp_env::with_vars;

    use super::Config;

    #[test]
    fn loads_defaults_when_database_url_set() {
        with_vars(
            [
                ("ISSUEBOSS__DATABASE_URL", Some("postgres://localhost/test")),
                ("ISSUEBOSS__HTTP_PORT", None),
                ("ISSUEBOSS__MCP_PORT", None),
                ("ISSUEBOSS__GRPC_PORT", None),
            ],
            || {
                let cfg = Config::load().expect("should load with database_url set");
                assert_eq!(cfg.http_port, 8080);
                assert_eq!(cfg.mcp_port, 8090);
                assert_eq!(cfg.grpc_port, 9090);
                assert_eq!(cfg.database_url, "postgres://localhost/test");
            },
        );
    }

    #[test]
    fn fails_without_database_url() {
        with_vars([("ISSUEBOSS__DATABASE_URL", None::<&str>)], || {
            assert!(Config::load().is_err(), "must fail without database_url");
        });
    }

    #[test]
    fn overrides_default_ports_via_env() {
        with_vars(
            [
                ("ISSUEBOSS__DATABASE_URL", Some("sqlite://:memory:")),
                ("ISSUEBOSS__HTTP_PORT", Some("9001")),
                ("ISSUEBOSS__GRPC_PORT", Some("9999")),
            ],
            || {
                let cfg = Config::load().expect("should load");
                assert_eq!(cfg.http_port, 9001);
                assert_eq!(cfg.grpc_port, 9999);
            },
        );
    }
}
