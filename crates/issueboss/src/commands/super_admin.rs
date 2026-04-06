use ib_api::grpc::admin::super_admin;

#[derive(Debug, clap::Args)]
pub(crate) struct SuperAdminArgs {
    /// Username for the SuperAdmin account
    #[arg(long)]
    pub username: String,

    /// Full name for the SuperAdmin account
    #[arg(long)]
    pub full_name: String,

    /// Email address for the SuperAdmin account
    #[arg(long)]
    pub email: String,

    /// Password for the SuperAdmin account
    #[arg(long)]
    pub password: String,
}

pub(crate) async fn cmd_super_admin(host: &str, port: u16, args: SuperAdminArgs) -> anyhow::Result<()> {
    let response = super_admin::api::super_admin(host, port, &args.username, &args.full_name, &args.password, &args.email)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    println!("\nSuperAdmin user created.\n");
    println!("  Username: {}", response.username);
    println!("  Email:    {}", response.email);
    println!("  API key:  {}", response.api_key);
    println!("\nStore this key securely — it will not be shown again.");
    println!("Set it as ISSUEBOSS_API_KEY to use the admin CLI.");
    Ok(())
}
