mod broker;
mod connectors;
mod dispatch;
mod github;
mod liveness;
mod resolver;
mod runner;
mod scheduler;
mod skills;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use orbit_events::{connect as connect_bus, EventBusConfig};
use orbit_harness::HarnessRegistry;
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
    /// Harness kinds this worker may claim. Enables separate harness images/workers.
    pub harnesses: Vec<String>,
    pub heartbeat_interval: Duration,
    pub stale_after: Duration,
    pub execution_driver: String,
    pub target: String,
    pub profile_id: Option<uuid::Uuid>,
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
            harnesses: std::env::var("ORBIT_WORKER_HARNESSES")
                .unwrap_or_else(|_| "mock,opencode,claude_code,pi,generic".into())
                .split(',')
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect(),
            heartbeat_interval: Duration::from_secs(
                std::env::var("ORBIT_WORKER_HEARTBEAT_SECONDS")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(10)
                    .max(2),
            ),
            stale_after: Duration::from_secs(
                std::env::var("ORBIT_WORKER_STALE_SECONDS")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(90)
                    .max(15),
            ),
            execution_driver: std::env::var("ORBIT_EXECUTION_DRIVER")
                .unwrap_or_else(|_| "process".into()),
            target: std::env::var("ORBIT_WORKER_TARGET").unwrap_or_else(|_| "local".into()),
            profile_id: std::env::var("ORBIT_WORKER_PROFILE_ID")
                .ok()
                .and_then(|value| uuid::Uuid::parse_str(&value).ok()),
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
    let registry = Arc::new(HarnessRegistry::builtin());

    // On a fresh Helm install the worker may start before the API has migrated and
    // seeded the default organization. Wait instead of crash-looping; the container then
    // becomes ready as soon as bootstrap finishes.
    let mut organization_found = false;
    for attempt in 1..=45 {
        match store.first_organization().await {
            Ok(Some(_)) => {
                organization_found = true;
                break;
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(error = %err, attempt, "database not ready while waiting for bootstrap");
            }
        }
        tracing::info!(attempt, "waiting for orbit-api to bootstrap the database");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    if !organization_found {
        anyhow::bail!(
            "no organization exists after waiting 90s; check that orbit-api started and migrated the database"
        );
    }

    let default_org = store
        .first_organization()
        .await?
        .map(|org| org.id)
        .context("organization disappeared during startup")?;

    let mut harnesses = config.harnesses.clone();
    let mut target = config.target.clone();
    let mut execution_driver = config.execution_driver.clone();
    let mut profile_agent_ids = Vec::new();
    let mut profile_skill_slugs = Vec::new();
    let mut profile_org_id = default_org;
    if let Some(profile_id) = config.profile_id {
        let profile = store.get_worker_profile_by_id(profile_id).await?;
        if !profile.enabled {
            anyhow::bail!("worker profile '{}' is disabled", profile.name);
        }
        harnesses = vec![profile.harness_kind.clone()];
        target = profile.target.clone();
        execution_driver = profile.execution_driver.clone();
        profile_agent_ids = profile.agent_ids.clone();
        profile_skill_slugs = profile.skill_slugs.clone();
        profile_org_id = profile.org_id;
        tracing::info!(
            profile = %profile.name,
            harness = %profile.harness_kind,
            target = %profile.target,
            agents = profile.agent_ids.len(),
            skills = profile.skill_slugs.len(),
            "worker profile loaded"
        );
    }

    let state = runner::WorkerState {
        worker_id: config.worker_id.clone(),
        store: store.clone(),
        bus: bus.clone(),
        registry: registry.clone(),
        workspace_root: std::path::PathBuf::from(&config.workspace_root),
        harnesses,
        execution_driver,
        target,
        profile_org_id,
        profile_agent_ids,
        profile_skill_slugs,
        active: Arc::new(tokio::sync::Mutex::new(std::collections::BTreeMap::new())),
    };

    tracing::info!(
        worker_id = %config.worker_id,
        concurrency = config.concurrency,
        scheduler = config.scheduler_enabled,
        harnesses = ?state.harnesses,
        events_driver = %bus_config.driver,
        "ORBIT worker started"
    );

    // Per-execution container mode: run exactly the assigned execution and exit.
    if let Ok(execution_id) = std::env::var("ORBIT_SINGLE_EXECUTION_ID") {
        let execution_id = uuid::Uuid::parse_str(&execution_id)
            .context("ORBIT_SINGLE_EXECUTION_ID must be a UUID")?;
        tracing::info!(%execution_id, worker_id = %state.worker_id, "single-execution worker starting");
        state
            .store
            .upsert_worker_heartbeat(
                &state.worker_id,
                &state.harnesses,
                &state.target,
                std::env::var("HOSTNAME").ok().as_deref(),
            )
            .await?;
        let result = runner::run_single(state.clone(), execution_id).await;
        let _ = state.store.delete_worker_heartbeat(&state.worker_id).await;
        result?;
        tracing::info!(%execution_id, "single-execution worker finished");
        return Ok(());
    }

    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    if config.scheduler_enabled {
        let scheduler_state = state.clone();
        tokio::spawn(async move {
            scheduler::run(scheduler_state).await;
        });
    }
    liveness::spawn(state.clone(), config.heartbeat_interval, config.stale_after);

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
    if let Err(error) = state.store.delete_worker_heartbeat(&state.worker_id).await {
        tracing::warn!(%error, "failed to remove worker heartbeat");
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
