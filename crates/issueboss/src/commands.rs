pub(crate) mod server;

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
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum Commands {
    #[command(about = "Start server", display_order = 10)]
    Server,
}
