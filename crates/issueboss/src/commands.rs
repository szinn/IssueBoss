pub(crate) mod server;
pub(crate) mod super_admin;

#[derive(Debug, clap::Parser)]
#[command(
    name = "IssueBoss",
    help_template = r#"
{before-help}{name} {version} - {about}

{usage-heading} {usage}

{all-args}{after-help}

AUTHORS:
    {author}
"#,
    version,
    author
)]
#[command(about, long_about = None)]
#[command(propagate_version = true, arg_required_else_help = true)]
pub(crate) struct CommandLine {
    /// gRPC server host (include scheme: http://localhost or https://prod.example.com)
    #[arg(long, global = true, env = "ISSUEBOSS_GRPC_HOST", default_value = "http://localhost")]
    pub host: String,

    /// gRPC server port
    #[arg(long, global = true, env = "ISSUEBOSS_GRPC_PORT", default_value_t = 9090u16)]
    pub port: u16,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Commands {
    #[command(about = "Start server", display_order = 10)]
    Server,

    #[command(about = "Bootstrap the SuperAdmin user", display_order = 20, name = "super-admin")]
    SuperAdmin(super_admin::SuperAdminArgs),
}
