use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use orbit_contracts::{
    subjects, ExecutionOutcome, ExecutionRequest, InstructionsBlock, OrbitEvent, RuntimeError,
    RuntimeErrorKind, RuntimeEvent, RuntimeEventKind, SessionRequest,
};
use orbit_domain::{
    AutonomyLevel, ExecutionEventKind, ExecutionStatus, GrantedCapability, LogLevel,
    ResourceLimits, RuntimeKind,
};
use orbit_store::{ExecutionRecord, Store};
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

/// Shared worker state.
#[derive(Clone)]
pub struct WorkerState {
    pub worker_id: String,
    pub store: Store,
    pub bus: Arc<dyn orbit_events::EventBus>,
    pub registry: Arc<orbit_runtime::RuntimeRegistry>,
    pub workspace_root: std::path::PathBuf,
    /// Runtime kinds this worker is allowed to claim.
    pub runtimes: Vec<String>,
    pub active: Arc<
        tokio::sync::Mutex<std::collections::BTreeMap<Uuid, tokio_util::sync::CancellationToken>>,
    >,
}

/// Poll the durable queue and dispatch executions, respecting a concurrency limit.
pub async fn claim_and_dispatch(
    state: &WorkerState,
    semaphore: &Arc<Semaphore>,
    poll_interval: Duration,
) -> anyhow::Result<()> {
    let mut dispatched = 0usize;
    loop {
        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => break,
        };
        let claimed = match state
            .store
            .claim_next_queued(&state.worker_id, &state.runtimes)
            .await
        {
            Ok(Some(execution)) => execution,
            Ok(None) => break,
            Err(err) => {
                drop(permit);
                return Err(err.into());
            }
        };
        dispatched += 1;
        let state = state.clone();
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(err) = run_execution(state, claimed).await {
                tracing::error!(error = %err, "execution runner failed");
            }
        });
    }

    if dispatched == 0 {
        tokio::time::sleep(poll_interval).await;
    }
    Ok(())
}

/// Execute one claimed execution and persist its lifecycle.
pub async fn run_execution(state: WorkerState, execution: ExecutionRecord) -> anyhow::Result<()> {
    let org_id = execution.org_id;
    let execution_id = execution.id;
    let spec_version = state
        .store
        .get_agent_version_by_id(org_id, execution.agent_version_id)
        .await?;
    let spec: orbit_domain::AgentSpec = serde_json::from_value(spec_version.spec.clone())
        .map_err(|e| anyhow::anyhow!("stored agent spec is invalid: {e}"))?;

    let runtime_kind = spec.runtime.kind;
    if let Some(record) = state
        .store
        .get_runtime_by_kind(org_id, runtime_kind.as_str())
        .await?
    {
        if !record.enabled {
            return fail_execution(
                &state,
                &execution,
                RuntimeError::new(
                    RuntimeErrorKind::Unavailable,
                    format!("runtime '{runtime_kind}' is disabled"),
                ),
            )
            .await;
        }
    }

    let adapter = match state.registry.require(runtime_kind) {
        Ok(adapter) => adapter,
        Err(err) => return fail_execution(&state, &execution, err).await,
    };

    let model = crate::resolver::resolve_model(&state.store, org_id, &spec)
        .await
        .map_err(|message| anyhow::anyhow!(message));
    let model = match model {
        Ok(model) => model,
        Err(err) => {
            return fail_execution(
                &state,
                &execution,
                RuntimeError::new(RuntimeErrorKind::InvalidConfig, err.to_string()),
            )
            .await
        }
    };

    let mut input: orbit_contracts::ExecutionInput =
        match serde_json::from_value(execution.input.clone()) {
            Ok(input) => input,
            Err(err) => {
                return fail_execution(
                    &state,
                    &execution,
                    RuntimeError::new(
                        RuntimeErrorKind::InvalidConfig,
                        format!("execution input is invalid: {err}"),
                    ),
                )
                .await
            }
        };

    // Read-only connector hydration (MVP: X recent search). External content is added
    // as untrusted payload; trusted instructions remain separate.
    match crate::connectors::hydrate(&state.store, org_id, &spec, &input.task, &input.payload).await
    {
        Ok(hydrated) => {
            input.payload = hydrated.payload;
            for note in hydrated.notes {
                append_event(
                    &state,
                    &execution,
                    ExecutionEventKind::Log,
                    LogLevel::Info,
                    &note,
                    serde_json::json!({ "connector": "x" }),
                )
                .await;
            }
        }
        Err(error) => return fail_execution(&state, &execution, error).await,
    }

    let grants = resolve_grants(&state.store, org_id, &spec).await;
    let workspace = state.workspace_root.join(execution_id.to_string());

    // Policy-enforcing tool broker: only for runtimes that can consume MCP today
    // (OpenCode). It is execution-scoped and dropped at the end of this function.
    let broker = if runtime_kind == RuntimeKind::OpenCode {
        match crate::broker::start(state.store.clone(), &spec, &execution).await {
            Ok(broker) => broker,
            Err(error) => {
                return fail_execution(
                    &state,
                    &execution,
                    RuntimeError::new(
                        RuntimeErrorKind::InvalidConfig,
                        format!("tool broker could not start: {error}"),
                    ),
                )
                .await
            }
        }
    } else {
        None
    };
    if let Some(handle) = &broker {
        append_event(
            &state,
            &execution,
            ExecutionEventKind::Status,
            LogLevel::Info,
            &format!(
                "policy-enforcing tool broker ready ({} GitHub tools)",
                handle.tool_count
            ),
            serde_json::json!({ "broker": "github", "tools": handle.tool_count }),
        )
        .await;
    }
    if let Err(err) = tokio::fs::create_dir_all(&workspace).await {
        tracing::warn!(error = %err, workspace = %workspace.display(), "could not create workspace");
    }

    let (event_tx, mut event_rx) = mpsc::channel::<RuntimeEvent>(256);
    let cancel = tokio_util::sync::CancellationToken::new();
    state
        .active
        .lock()
        .await
        .insert(execution_id, cancel.clone());

    let _ = state
        .store
        .mark_execution_running(execution_id, None)
        .await?;
    append_event(
        &state,
        &execution,
        ExecutionEventKind::Status,
        LogLevel::Info,
        "execution dispatched to runtime",
        serde_json::json!({ "runtime": runtime_kind.as_str(), "worker": state.worker_id }),
    )
    .await;

    // Persist runtime events as they arrive.
    let event_state = state.clone();
    let event_execution = execution.clone();
    let event_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let (kind, level, message, data) = map_runtime_event(event);
            append_event(&event_state, &event_execution, kind, level, &message, data).await;
        }
    });

    // Deliver cancellation requests from the control plane.
    let watcher_state = state.clone();
    let watcher_cancel = cancel.clone();
    let done = Arc::new(AtomicBool::new(false));
    let watcher_done = done.clone();
    let watcher = tokio::spawn(async move {
        while !watcher_done.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            match watcher_state.store.is_cancel_requested(execution_id).await {
                Ok(true) => {
                    tracing::info!(%execution_id, "cancellation requested; cancelling runtime");
                    watcher_cancel.cancel();
                    break;
                }
                Ok(false) => {}
                Err(err) => tracing::warn!(error = %err, "cancel poll failed"),
            }
        }
    });

    // Add the MCP bridge details to the runtime config. This stays in memory only; the
    // execution-scoped token is invalid as soon as the broker is dropped.
    let mut runtime_config = spec.runtime.config.clone();
    if let (Some(handle), RuntimeKind::OpenCode) = (&broker, runtime_kind) {
        if !runtime_config.is_object() {
            runtime_config = serde_json::json!({});
        }
        runtime_config["orbitMcp"] = serde_json::json!({
            "command": handle.mcp_command(),
            "environment": {
                "ORBIT_BROKER_URL": handle.url,
                "ORBIT_BROKER_TOKEN": handle.token,
            }
        });
    }

    let autonomy = AutonomyLevel::from_level(spec.autonomy.level).unwrap_or(AutonomyLevel::OBSERVE);
    let request = ExecutionRequest {
        execution_id,
        agent_id: execution.agent_id,
        agent_version_id: execution.agent_version_id,
        agent_name: execution.agent_name.clone(),
        instructions: InstructionsBlock {
            system: spec.instructions.system.clone(),
            constraints: spec.instructions.constraints.clone(),
        },
        input,
        model,
        grants,
        autonomy,
        resources: ResourceLimits {
            timeout_seconds: execution.timeout_seconds.max(1) as u64,
            ..spec.resources.clone()
        },
        runtime_config,
        session: SessionRequest {
            session_ref: execution.session_ref.clone(),
            mode: spec.runtime.execution_mode,
        },
        workspace: orbit_contracts::WorkspaceSpec {
            root: workspace.to_string_lossy().to_string(),
            read_only: false,
        },
        events: Some(event_tx),
        cancel: cancel.clone(),
    };

    let timeout_seconds = request.resources.timeout_seconds;
    let started = std::time::Instant::now();
    let outcome = tokio::select! {
        _ = cancel.cancelled() => {
            let mut outcome = ExecutionOutcome::failed(RuntimeError::cancelled());
            outcome.status = ExecutionStatus::Cancelled;
            outcome
        }
        result = tokio::time::timeout(
            Duration::from_secs(timeout_seconds.saturating_add(5)),
            adapter.execute(request),
        ) => match result {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(err)) => outcome_from_error(err),
            Err(_) => {
                let mut outcome = ExecutionOutcome::failed(RuntimeError::timeout(timeout_seconds));
                outcome.status = ExecutionStatus::TimedOut;
                outcome
            }
        }
    };

    // Stop the cancellation watcher before finishing.
    done.store(true, Ordering::Relaxed);
    watcher.abort();
    state.active.lock().await.remove(&execution_id);

    // Drain the event channel: drop the sender by ending the adapter future, then wait.
    // `event_tx` lives inside the request which has been consumed above.
    let _ = tokio::time::timeout(Duration::from_secs(2), event_task).await;

    let mut outcome = outcome;
    if outcome.usage.wall_time_ms.is_none() {
        outcome.usage.wall_time_ms = Some(started.elapsed().as_millis() as i64);
    }

    let status = if outcome.status.is_terminal() {
        outcome.status
    } else {
        ExecutionStatus::Succeeded
    };
    let error_json = outcome
        .error
        .as_ref()
        .and_then(|error| serde_json::to_value(error).ok());

    append_event(
        &state,
        &execution,
        if status == ExecutionStatus::Succeeded {
            ExecutionEventKind::Status
        } else {
            ExecutionEventKind::Error
        },
        if status == ExecutionStatus::Succeeded {
            LogLevel::Info
        } else {
            LogLevel::Error
        },
        &match status {
            ExecutionStatus::Succeeded => "execution succeeded".to_string(),
            other => format!("execution finished with status {other}"),
        },
        serde_json::json!({
            "status": status.as_str(),
            "usage": outcome.usage,
        }),
    )
    .await;

    let updated = state
        .store
        .finish_execution(
            execution_id,
            status,
            outcome.output.as_ref(),
            error_json.as_ref(),
            serde_json::to_value(&outcome.usage).ok().as_ref(),
            outcome.session_ref.as_deref(),
        )
        .await?;

    match updated {
        Some(record) => {
            publish_status(&state, &record).await;
            tracing::info!(
                %execution_id,
                status = status.as_str(),
                duration_ms = started.elapsed().as_millis() as u64,
                "execution finished"
            );
        }
        None => {
            tracing::warn!(%execution_id, "execution was already terminal; result discarded");
        }
    }
    Ok(())
}

fn outcome_from_error(error: RuntimeError) -> ExecutionOutcome {
    let status = match error.kind {
        RuntimeErrorKind::Cancelled => ExecutionStatus::Cancelled,
        RuntimeErrorKind::Timeout => ExecutionStatus::TimedOut,
        _ => ExecutionStatus::Failed,
    };
    let mut outcome = ExecutionOutcome::failed(error);
    outcome.status = status;
    outcome
}

async fn fail_execution(
    state: &WorkerState,
    execution: &ExecutionRecord,
    error: RuntimeError,
) -> anyhow::Result<()> {
    let outcome = outcome_from_error(error);
    append_event(
        state,
        execution,
        ExecutionEventKind::Error,
        LogLevel::Error,
        &outcome
            .error
            .as_ref()
            .map(|e| e.message.clone())
            .unwrap_or_else(|| "execution failed".into()),
        serde_json::json!({ "error": outcome.error }),
    )
    .await;
    let error_json = outcome
        .error
        .as_ref()
        .and_then(|e| serde_json::to_value(e).ok());
    let updated = state
        .store
        .finish_execution(
            execution.id,
            outcome.status,
            None,
            error_json.as_ref(),
            None,
            None,
        )
        .await?;
    if let Some(record) = updated {
        publish_status(state, &record).await;
    }
    Ok(())
}

async fn append_event(
    state: &WorkerState,
    execution: &ExecutionRecord,
    kind: ExecutionEventKind,
    level: LogLevel,
    message: &str,
    data: serde_json::Value,
) {
    let result = state
        .store
        .append_execution_event(
            execution.org_id,
            execution.id,
            kind.as_str(),
            level,
            message,
            &data,
        )
        .await;
    match result {
        Ok(event) => {
            let envelope = OrbitEvent::new(
                subjects::EXECUTION_EVENT,
                "orbit-worker",
                serde_json::to_value(&event).unwrap_or_default(),
            )
            .with_org(execution.org_id)
            .with_idempotency(format!("event:{}", event.id));
            if let Err(err) = state.bus.publish(envelope).await {
                tracing::debug!(error = %err, "event publish failed (SSE reads the database)");
            }
        }
        Err(err) => tracing::error!(error = %err, "failed to persist execution event"),
    }
}

async fn publish_status(state: &WorkerState, execution: &ExecutionRecord) {
    let event = OrbitEvent::new(
        subjects::EXECUTION_UPDATED,
        "orbit-worker",
        serde_json::json!({
            "executionId": execution.id,
            "agentId": execution.agent_id,
            "status": execution.status,
            "finishedAt": execution.finished_at,
        }),
    )
    .with_org(execution.org_id)
    .with_idempotency(format!("status:{}:{}", execution.id, execution.status));
    if let Err(err) = state.bus.publish(event).await {
        tracing::debug!(error = %err, "status publish failed");
    }
}

fn map_runtime_event(
    event: RuntimeEvent,
) -> (ExecutionEventKind, LogLevel, String, serde_json::Value) {
    let kind = match event.kind {
        RuntimeEventKind::Started | RuntimeEventKind::Status => ExecutionEventKind::Status,
        RuntimeEventKind::Log => ExecutionEventKind::Log,
        RuntimeEventKind::ToolCall => ExecutionEventKind::ToolCall,
        RuntimeEventKind::OutputDelta => ExecutionEventKind::Output,
        RuntimeEventKind::Usage => ExecutionEventKind::Metric,
        RuntimeEventKind::Error => ExecutionEventKind::Error,
    };
    (kind, event.level, event.message, event.data)
}

async fn resolve_grants(
    store: &Store,
    org_id: Uuid,
    spec: &orbit_domain::AgentSpec,
) -> Vec<GrantedCapability> {
    let mut grants = Vec::new();
    for binding in &spec.connectors {
        let connector = match store
            .get_connector_by_slug(org_id, &binding.connector)
            .await
        {
            Ok(Some(connector)) => connector,
            Ok(None) => continue,
            Err(err) => {
                tracing::warn!(error = %err, connector = %binding.connector, "grant resolution failed");
                continue;
            }
        };
        for capability in &binding.capabilities {
            let effect = connector
                .capabilities
                .as_array()
                .and_then(|items| {
                    items.iter().find(|item| {
                        item.get("id").and_then(|id| id.as_str()) == Some(capability.as_str())
                    })
                })
                .and_then(|item| item.get("effect").and_then(|e| e.as_str()))
                .and_then(orbit_domain::CapabilityEffect::parse)
                .unwrap_or(orbit_domain::CapabilityEffect::Read);
            grants.push(GrantedCapability {
                connection_id: binding.connection_id.unwrap_or(Uuid::nil()),
                connector: binding.connector.clone(),
                capability: capability.clone(),
                effect,
            });
        }
    }
    grants
}
