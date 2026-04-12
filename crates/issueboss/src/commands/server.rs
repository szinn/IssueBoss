#[cfg(feature = "server")]
pub(crate) async fn cmd_server(config: crate::config::Config) -> anyhow::Result<()> {
    use anyhow::Context;
    use ib_core::create_services;
    use ib_database::{create_repository_service, open_database};
    use ib_frontend::create_frontend_subsystem;
    use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle, Toplevel};

    tracing::info!("IssueBoss {}", clap::crate_version!());
    let span = tracing::span!(tracing::Level::TRACE, "IssueBoss Startup").entered();

    use ib_api::create_api_subsystem;
    use tokio_graceful_shutdown::SubsystemBuilder;

    let database = open_database(&config.database_url).await.context("Couldn't create database connection")?;
    let repository_service = create_repository_service(database).await.context("Couldn't create database connection")?;

    let core_services = create_services(repository_service.clone());

    let (http_port, mcp_port, grpc_port) = (config.http_port, config.mcp_port, config.grpc_port);
    let api_subsystem = create_api_subsystem(grpc_port, mcp_port, core_services.clone());
    let frontend_subsystem = create_frontend_subsystem(http_port, core_services.clone());

    span.exit();

    Toplevel::new(async |s: &mut SubsystemHandle| {
        s.start(SubsystemBuilder::new("Api", api_subsystem.into_subsystem()));
        s.start(SubsystemBuilder::new("Frontend", frontend_subsystem.into_subsystem()));
    })
    .catch_signals()
    .handle_shutdown_requests(std::time::Duration::from_secs(5))
    .await?;

    Ok(())
}
