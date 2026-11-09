mod resolver;
mod runner;
mod scheduler;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use orbit_events::{connect as connect_bus, EventBusConfig};
use orbit_runtime::RuntimeRegistry;
use orbit_security::SecretCipher;
use orbit_store::Store;
use tokio::sync::Semaphore;

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub worker_id: String,
    pub database_url: String,
    pub db_max_connections: u32,
    pub poll_interval: Duration,
    pub scheduler_enabled: bool,
    pub concurrency: usize,
    pub workspace_root: String,
}

impl WorkerConfig {
    fn from_env() -> anyhow::Result<Self> {
        let worker_id = std::env::var("ORBIT_WORKER_ID").unwrap_or_else(|_| {
            format!(
                "worker-{}",
                std::env::var("HOSTNAME").unwrap_or_else(|_| uuid::Uuid::now_v7().to_string())
            )
        });
        Ok(Self {
            worker_id,
            database_url: std::env::var("ORBIT_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://orbit:orbit@localhost:5432/orbit".into()),
            db_max_connections: std::env::var("ORBIT_DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            poll_interval: Duration::from_millis(
                std::env::var("ORBIT_WORKER_POLL_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1000),
            ),
            scheduler_enabled: orbit_api_telemetry_env_bool("ORBIT_SCHEDULER_ENABLED", true),
            concurrency: std::env::var("ORBIT_WORKER_CONCURRENCY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4)
                .max(1),
            workspace_root: std::env::var("ORBIT_WORKSPACE_ROOT")
                .unwrap_or_else(|_| "/tmp/orbit-workspaces".into()),
        })
    }
}

fn orbit_api_telemetry_env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes"))
        .unwrap_or(default)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    orbit_api::telemetry::init("orbit-worker");

    let config = WorkerConfig::from_env().context("invalid worker configuration")?;
    if std::env::var("ORBIT_ENV").as_deref() == Ok("production")
        && std::env::var("ORBIT_EVENTS_DRIVER").as_deref() == Ok("memory")
    {
        anyhow::bail!("the in-memory event bus is not allowed in production");
    }

    let master_key = std::env::var("ORBIT_MASTER_KEY").unwrap_or_default();
    let cipher = if master_key.trim().is_empty() {
        tracing::warn!("ORBIT_MASTER_KEY is not set; the worker cannot resolve stored credentials");
        None
    } else {
        Some(Arc::new(
            SecretCipher::from_base64_key(&master_key).context("invalid ORBIT_MASTER_KEY")?,
        ))
    };
    let store = Store::connect(&config.database_url, config.db_max_connections)
        .await
        .context("could not connect to PostgreSQL")?;
    let store = match cipher {
        Some(cipher) => store.with_cipher(cipher),
        None => store,
    };
    if std::env::var("ORBIT_ENV").as_deref() != Ok("production") {
        store.migrate().await.ok();
    }

    let bus_config = EventBusConfig::from_env().context("invalid event bus configuration")?;
    let bus = connect_bus(&bus_config)
        .await
        .context("could not connect to the event bus")?;
    let registry = Arc::new(RuntimeRegistry::builtin());

    if store.first_organization().await?.is_none() {
        anyhow::bail!("no organization exists; start orbit-api once to bootstrap the database");
    }

    let state = runner::WorkerState {
        worker_id: config.worker_id.clone(),
        store: store.clone(),
        bus: bus.clone(),
        registry: registry.clone(),
        workspace_root: std::path::PathBuf::from(&config.workspace_root),
        active: Arc::new(tokio::sync::Mutex::new(std::collections::BTreeMap::new())),
    };

    tracing::info!(
        worker_id = %config.worker_id,
        concurrency = config.concurrency,
        scheduler = config.scheduler_enabled,
        events_driver = %bus_config.driver,
        "ORBIT worker started (mock runtime available; OpenCode adapter scaffolded)"
    );

    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    if config.scheduler_enabled {
        let scheduler_state = state.clone();
        tokio::spawn(async move {
            scheduler::run(scheduler_state).await;
        });
    }

    loop {
        tokio::select! {
            _ = shutdown_signal() => {
                tracing::info!("shutdown signal received; waiting for active executions");
                break;
            }
            result = runner::claim_and_dispatch(&state, &semaphore, config.poll_interval) => {
                if let Err(err) = result {
                    tracing::error!(error = %err, "dispatch loop error");
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    // Give in-flight executions a bounded grace period.
    for _ in 0..30 {
        let active = state.active.lock().await.len();
        if active == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    tracing::info!("ORBIT worker stopped");
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
}
