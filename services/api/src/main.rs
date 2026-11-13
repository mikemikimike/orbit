use std::sync::Arc;

use anyhow::Context;
use orbit_api::{build_router, initialize, telemetry, ApiConfig, AppState};
use orbit_events::{connect as connect_bus, EventBusConfig};
use orbit_harness::HarnessRegistry;
use orbit_security::{JwtValidator, SecretCipher};
use orbit_store::Store;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    telemetry::init("orbit-api");

    let config = ApiConfig::from_env().context("invalid API configuration")?;
    if config.is_production() {
        if matches!(config.auth.mode, orbit_security::AuthMode::Dev) {
            anyhow::bail!("ORBIT_AUTH_MODE=dev is not allowed in production");
        }
        if config.cors_origins.iter().any(|origin| origin == "*") {
            tracing::warn!("CORS is configured to allow any origin in production");
        }
    }

    let cipher = Arc::new(
        SecretCipher::from_base64_key(&config.master_key)
            .context("ORBIT_MASTER_KEY must be a base64-encoded 32-byte key")?,
    );
    let store = Store::connect(&config.database_url, config.db_max_connections)
        .await
        .context("could not connect to PostgreSQL")?
        .with_cipher(cipher);

    let bus_config = EventBusConfig::from_env().context("invalid event bus configuration")?;
    let bus = connect_bus(&bus_config)
        .await
        .context("could not connect to the event bus")?;
    let registry = Arc::new(HarnessRegistry::builtin());

    let initialized = initialize(&store, &registry, &config, bus.as_ref())
        .await
        .context("bootstrap failed")?;

    // Settings saved through the UI are applied on startup (restart to take effect).
    let auth_config =
        orbit_api::handlers::settings::apply_stored_oidc(&store, config.auth.clone()).await;

    let jwt = match auth_config.mode {
        orbit_security::AuthMode::Oidc => {
            let oidc = auth_config
                .oidc
                .as_ref()
                .context("OIDC configuration missing")?;
            Some(Arc::new(
                JwtValidator::new(oidc).context("invalid OIDC config")?,
            ))
        }
        orbit_security::AuthMode::Dev => None,
    };

    let state = AppState {
        store,
        bus,
        registry,
        auth: Arc::new(auth_config),
        jwt,
        default_org_id: initialized.default_org_id,
        dev_identity: initialized.dev_identity.map(Arc::new),
        environment: config.environment.clone(),
        events_driver: bus_config.driver.clone(),
        public_base_url: config.public_base_url.clone(),
        metrics: Arc::new(orbit_api::metrics::Metrics::new()),
        max_body_bytes: config.body_limit_bytes,
        cors_origins: config.cors_origins.clone(),
        launcher_url: config.launcher_url.clone(),
        launcher_token: config.launcher_token.clone(),
        worker_api_url: config.worker_api_url.clone(),
        archive_after_days: config.archive_after_days,
    };

    orbit_api::background::spawn(state.clone());
    let router = build_router(state);
    let listener = TcpListener::bind(&config.http_addr)
        .await
        .with_context(|| format!("could not bind {}", config.http_addr))?;
    tracing::info!(
        address = %config.http_addr,
        auth_mode = config.auth.mode.as_str(),
        events_driver = %bus_config.driver,
        "ORBIT API listening"
    );

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("HTTP server failed")?;
    tracing::info!("ORBIT API stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("ctrl-c handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
