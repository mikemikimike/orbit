//! Per-execution container dispatch.
//!
//! Instead of running the runtime inside the long-lived worker process, the parent
//! claims the execution and starts a short-lived container running `orbit-worker` in
//! single-execution mode. The child connects to PostgreSQL/NATS, runs exactly one
//! execution (including its own tool broker/MCP bridge), writes events and exits.
//!
//! Docker socket access is root-equivalent. This driver is for single-user/homelab
//! deployments; Kubernetes Job dispatch is the production-grade equivalent.

use std::time::Duration;

use chrono::Utc;
use orbit_contracts::{RuntimeError, RuntimeErrorKind};
use orbit_store::ExecutionRecord;
use tokio::process::Command;

use crate::runner::WorkerState;

fn image_for(runtime: &str) -> Option<String> {
    let key = format!(
        "ORBIT_EXECUTION_IMAGE_{}",
        runtime.to_uppercase().replace('-', "_")
    );
    std::env::var(&key)
        .ok()
        .or_else(|| std::env::var("ORBIT_EXECUTION_IMAGE").ok())
        .filter(|value| !value.trim().is_empty())
}

fn rewrite_host(url: &str) -> String {
    if std::env::var("ORBIT_DISPATCH_REWRITE_HOSTS")
        .ok()
        .as_deref()
        == Some("false")
    {
        return url.to_string();
    }
    url.replace("//localhost:", "//host.docker.internal:")
        .replace("//127.0.0.1:", "//host.docker.internal:")
        // postgres URLs put credentials before the host: user:pass@localhost:5432
        .replace("@localhost:", "@host.docker.internal:")
        .replace("@127.0.0.1:", "@host.docker.internal:")
}

pub async fn run(state: &WorkerState, execution: &ExecutionRecord) -> anyhow::Result<()> {
    let Some(image) = image_for(&execution.runtime_kind) else {
        crate::runner::fail_execution(
            state,
            execution,
            RuntimeError::new(
                RuntimeErrorKind::InvalidConfig,
                format!(
                    "docker execution driver has no image for runtime '{}' (set ORBIT_EXECUTION_IMAGE or ORBIT_EXECUTION_IMAGE_{})",
                    execution.runtime_kind,
                    execution.runtime_kind.to_uppercase()
                ),
            ),
        )
        .await?;
        return Ok(());
    };

    let docker = std::env::var("ORBIT_DOCKER_BIN").unwrap_or_else(|_| "docker".into());
    let container_name = format!(
        "orbit-exec-{}-{}",
        &execution.id.to_string()[..8],
        Utc::now().timestamp_millis()
    );
    let worker_id = format!("container-{container_name}");
    let database_url = rewrite_host(
        &std::env::var("ORBIT_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://orbit:orbit@localhost:5432/orbit".into()),
    );
    let nats_url = rewrite_host(
        &std::env::var("ORBIT_NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into()),
    );

    let mut args: Vec<String> = vec![
        "run".into(),
        "--rm".into(),
        "--name".into(),
        container_name.clone(),
        "--add-host".into(),
        "host.docker.internal:host-gateway".into(),
    ];

    let auth_export = std::env::var("ORBIT_OPENCODE_AUTH_EXPORT").ok();
    if execution.runtime_kind == "opencode" {
        if let Some(path) = &auth_export {
            if std::path::Path::new(path).exists() {
                args.push("-v".into());
                args.push(format!("{path}:/seed/auth-export.json:ro"));
            }
        }
    }

    let envs: Vec<(&str, String)> = vec![
        (
            "ORBIT_ENV",
            std::env::var("ORBIT_ENV").unwrap_or_else(|_| "development".into()),
        ),
        ("ORBIT_DATABASE_URL", database_url),
        ("ORBIT_NATS_URL", nats_url),
        (
            "ORBIT_MASTER_KEY",
            std::env::var("ORBIT_MASTER_KEY").unwrap_or_default(),
        ),
        (
            "ORBIT_EVENTS_DRIVER",
            std::env::var("ORBIT_EVENTS_DRIVER").unwrap_or_else(|_| "nats".into()),
        ),
        ("ORBIT_SINGLE_EXECUTION_ID", execution.id.to_string()),
        ("ORBIT_WORKER_ID", worker_id.clone()),
        ("ORBIT_WORKER_RUNTIMES", execution.runtime_kind.clone()),
        ("ORBIT_WORKER_TARGET", "docker".into()),
        ("ORBIT_SCHEDULER_ENABLED", "false".into()),
        ("ORBIT_EXECUTION_DRIVER", "process".into()),
        ("ORBIT_WORKER_POLL_MS", "1000".into()),
        ("ORBIT_WORKSPACE_ROOT", "/tmp/orbit-workspaces".into()),
        (
            "ORBIT_MCP_BROKER_BIN",
            "/usr/local/bin/orbit-mcp-broker".into(),
        ),
        (
            "ORBIT_LOG_FORMAT",
            std::env::var("ORBIT_LOG_FORMAT").unwrap_or_else(|_| "json".into()),
        ),
    ];
    for (key, value) in &envs {
        if value.is_empty() {
            continue;
        }
        args.push("-e".into());
        args.push(format!("{key}={value}"));
    }

    // Override the image entrypoint so the auth import wrapper runs and then execs the
    // worker binary. Without this, `sh -c ...` would be passed as arguments to the
    // worker entrypoint and the import would be skipped.
    args.push("--entrypoint".into());
    args.push("/bin/sh".into());
    args.push(image.clone());
    args.push("-c".into());
    args.push(
        "mkdir -p \"$HOME/.local/share/opencode\"; \
         if [ -n \"$XDG_CACHE_HOME\" ]; then mkdir -p \"$XDG_CACHE_HOME\"; fi; \
         if command -v opencode >/dev/null 2>&1 && [ -f /seed/auth-export.json ]; then \
           opencode auth import --standalone /seed/auth-export.json || true; \
         fi; \
         exec /usr/local/bin/orbit-worker"
            .into(),
    );

    tracing::info!(
        execution_id = %execution.id,
        runtime = %execution.runtime_kind,
        image = %image,
        container = %container_name,
        "dispatching execution to a per-execution container"
    );

    let mut command = Command::new(&docker);
    command.args(&args);
    // Inherit logs instead of piping: a per-execution container can produce a lot of
    // output, and an undrained pipe would deadlock the parent.
    command.stdout(std::process::Stdio::inherit());
    command.stderr(std::process::Stdio::inherit());
    let mut child = command.spawn().map_err(|error| {
        anyhow::anyhow!("could not start docker for per-execution container: {error}")
    })?;

    let wait = child.wait();
    let timeout = Duration::from_secs(execution.timeout_seconds.max(1) as u64 + 180);
    let status = match tokio::time::timeout(timeout, wait).await {
        Ok(result) => result?,
        Err(_) => {
            let _ = Command::new(&docker)
                .args(["rm", "-f", &container_name])
                .status()
                .await;
            crate::runner::fail_execution(
                state,
                execution,
                RuntimeError::timeout(execution.timeout_seconds as u64),
            )
            .await?;
            return Ok(());
        }
    };

    let stderr = String::new();
    if !status.success() {
        let message = format!(
            "per-execution container exited with {}: {}",
            status,
            stderr.trim()
        );
        tracing::warn!(execution_id = %execution.id, %message, "container execution failed");
        let current = state.store.get_execution_by_id(execution.id).await?;
        if !matches!(
            orbit_domain::ExecutionStatus::parse(&current.status),
            Some(status) if status.is_terminal()
        ) {
            crate::runner::fail_execution(
                state,
                execution,
                RuntimeError::new(RuntimeErrorKind::ExecutionFailed, message),
            )
            .await?;
        }
    } else {
        tracing::info!(execution_id = %execution.id, "per-execution container finished");
    }
    Ok(())
}
