#[cfg(feature = "server")]
pub(crate) async fn cmd_server(config: crate::config::Config) -> anyhow::Result<()> {
    use anyhow::Context;
    use ib_core::create_services;
    use ib_database::{create_repository_service, open_database};
    use tokio_graceful_shutdown::{IntoSubsystem, SubsystemHandle, Toplevel};

    tracing::info!("IssueBoss {}", clap::crate_version!());
    let span = tracing::span!(tracing::Level::TRACE, "IssueBoss Startup").entered();

    use ib_api::create_api_subsystem;
    use tokio_graceful_shutdown::SubsystemBuilder;

    let database = open_database(&config.database_url).await.context("Couldn't create database connection")?;
    let repository_service = create_repository_service(database).await.context("Couldn't create database connection")?;

    let core_services = create_services(repository_service.clone());

    let (_http_port, mcp_port, grpc_port) = (config.http_port, config.mcp_port, config.grpc_port);
    let api_subsystem = create_api_subsystem(grpc_port, mcp_port, core_services.clone());

    span.exit();

    Toplevel::new(async |s: &mut SubsystemHandle| {
        s.start(SubsystemBuilder::new("Api", api_subsystem.into_subsystem()));
        // s.start(SubsystemBuilder::new("frontend", move |subsys|
        // create_frontend_subsystem(http_port, subsys)));
    })
    .catch_signals()
    .handle_shutdown_requests(std::time::Duration::from_secs(5))
    .await?;

    // use std::{sync::Arc, time::Duration};
    //
    // use anyhow::Context;
    // use tokio_graceful_shutdown::{IntoSubsystem, SubsystemBuilder,
    // SubsystemHandle, Toplevel};
    //
    // tracing::info!("IssueBoss {}", clap::crate_version!());
    // let span = tracing::span!(tracing::Level::TRACE, "IssueBoss
    // Startup").entered();
    //
    // let database = open_database(&config.database).await.context("Couldn't create
    // database connection")?; let repository_service =
    // create_repository_service(database).await.context("Couldn't create database
    // connection")?; let file_store =
    // Arc::new(bb_storage::LocalFileStore::new(config.library.library_path.
    // clone())); let format_service: Arc<dyn FormatService> =
    // Arc::new(create_format_service()); let worker_poll_interval =
    // Duration::from_secs(config.import.worker_poll_interval_secs);
    //
    // let external = ExternalServicesBuilder::default()
    //     .repository_service(repository_service.clone())
    //     .file_store(file_store)
    //     .format_service(format_service)
    //     .bookdrop_path(config.import.bookdrop_path.clone())
    //     .scan_interval(Duration::from_secs(config.import.scan_interval_secs))
    //     .build()
    //     .context("ExternalServices missing required field")?;
    // let core_services = create_services(external,
    // &config.encryption_secret).context("Couldn't create core services")?;
    //
    // // Each crate self-registers its job handlers and health task configs.
    // bb_core::before_start(&core_services);
    // // Register configured metadata providers into the metadata service.
    // metadata_before_start(&core_services, &config.metadata);
    //
    // let api_subsystem = create_api_subsystem(&config.api, core_services.clone());
    // let core_subsystem = create_core_subsystem(core_services.clone(),
    // worker_poll_interval); let core_subsystem =
    // bb_core::ResilienceWrapper::new("Core", core_subsystem,
    // core_services.system_message_service.clone()); let frontend_subsystem =
    // create_frontend_subsystem(&config.frontend, core_services.clone());
    //
    // span.exit();
    //
    // Toplevel::new(async |s: &mut SubsystemHandle| {
    //     s.start(SubsystemBuilder::new("Api", api_subsystem.into_subsystem()));
    //     s.start(SubsystemBuilder::new("Core", core_subsystem.into_subsystem()));
    //     s.start(SubsystemBuilder::new("Frontend",
    // frontend_subsystem.into_subsystem())); })
    // .catch_signals()
    // .handle_shutdown_requests(Duration::from_secs(3))
    // .await?;
    //
    // repository_service.repository().close().await.context("Couldn't close
    // database")?;
    Ok(())
}
