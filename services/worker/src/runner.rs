use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use orbit_contracts::{
    ExecutionOutcome, ExecutionRequest, HarnessError, HarnessErrorKind, HarnessEvent,
    HarnessEventKind, InstructionsBlock, SessionRequest,
};
use orbit_domain::{
    AutonomyLevel, ExecutionEventKind, ExecutionStatus, HarnessKind, LogLevel, ResourceLimits,
};
use orbit_store::ExecutionRecord;
use tokio::sync::{mpsc, Semaphore};
use uuid::Uuid;

/// Shared worker state.
#[derive(Clone)]
pub struct WorkerState {
    pub worker_id: String,
    pub client: crate::client::WorkerClient,
    pub registry: Arc<orbit_harness::HarnessRegistry>,
    pub workspace_root: std::path::PathBuf,
    /// Harness kinds this worker is allowed to claim.
    pub harnesses: Vec<String>,
    /// `process` or `docker` (per-execution container).
    pub execution_driver: String,
    /// Worker target class (`local`, `docker`, `kubernetes`).
    pub target: String,
    /// Agents this worker is scoped to (empty = all).
    pub profile_agent_ids: Vec<Uuid>,
    /// Skills materialized for every execution handled by this worker.
    pub profile_skill_slugs: Vec<String>,
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
            .client
            .claim(&crate::client::ClaimRequest {
                worker_id: &state.worker_id,
                harnesses: &state.harnesses,
                target: &state.target,
                agent_ids: &state.profile_agent_ids,
            })
            .await
        {
            Ok(Some(execution)) => execution,
            Ok(None) => break,
            Err(err) => {
                drop(permit);
                return Err(err);
            }
        };
        dispatched += 1;
        let state = state.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let driver = state.execution_driver.clone();
            let result = if driver == "docker" {
                crate::dispatch::run(&state, &claimed).await
            } else {
                run_execution(state, claimed).await
            };
            if let Err(err) = result {
                tracing::error!(error = %err, "execution runner failed");
            }
        });
    }

    if dispatched == 0 {
        tokio::time::sleep(poll_interval).await;
    }
    Ok(())
}

/// Single-execution container mode: bind one dispatched execution to this worker and run
/// it, without entering the claim loop or scheduler.
pub async fn run_single(state: WorkerState, execution_id: uuid::Uuid) -> anyhow::Result<()> {
    let context = state.client.context(execution_id).await?;
    run_execution(state, context).await
}

/// Execute one claimed execution and persist its lifecycle.
pub async fn run_execution(
    state: WorkerState,
    context: crate::client::WorkerExecutionContext,
) -> anyhow::Result<()> {
    let execution = context.execution.clone();
    let execution_id = execution.id;
    let spec = context.spec;
    let harness_kind = spec.harness.kind;

    let adapter = match state.registry.require(harness_kind) {
        Ok(adapter) => adapter,
        Err(err) => return fail_execution(&state, &execution, err).await,
    };

    let model = context.model.map(|model| model.into_resolved());
    let mut input: orbit_contracts::ExecutionInput =
        match serde_json::from_value(execution.input.clone()) {
            Ok(input) => input,
            Err(err) => {
                return fail_execution(
                    &state,
                    &execution,
                    HarnessError::new(
                        HarnessErrorKind::InvalidConfig,
                        format!("execution input is invalid: {err}"),
                    ),
                )
                .await
            }
        };

    // Read-only connector hydration (MVP: X recent search). External content is added
    // as untrusted payload; trusted instructions remain separate.
    match crate::connectors::hydrate(
        &state.client,
        execution_id,
        &spec,
        &input.task,
        &input.payload,
    )
    .await
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

    let grants = context.grants;
    let workspace = state.workspace_root.join(execution_id.to_string());
    if let Err(err) = tokio::fs::create_dir_all(&workspace).await {
        tracing::warn!(error = %err, workspace = %workspace.display(), "could not create workspace");
    }
    let checkout_connection = spec
        .connectors
        .iter()
        .find(|binding| {
            binding.connector == "github"
                && binding
                    .capabilities
                    .iter()
                    .any(|capability| capability == "repo.checkout")
        })
        .and_then(|binding| binding.connection_id);
    match crate::git::checkout(
        &state.client,
        execution_id,
        checkout_connection,
        &execution.input,
        &workspace,
    )
    .await
    {
        Ok(Some(note)) => {
            append_event(
                &state,
                &execution,
                ExecutionEventKind::Status,
                LogLevel::Info,
                &note,
                serde_json::json!({ "connector": "github", "capability": "repo.checkout" }),
            )
            .await;
        }
        Ok(None) => {}
        Err(error) => {
            return fail_execution(
                &state,
                &execution,
                HarnessError::new(
                    HarnessErrorKind::InvalidConfig,
                    format!("repository checkout failed: {error}"),
                ),
            )
            .await
        }
    }

    // Policy-enforcing tool broker: only for harnesses that can consume MCP today
    // (OpenCode). It is execution-scoped and dropped at the end of this function.
    let broker = if matches!(harness_kind, HarnessKind::OpenCode | HarnessKind::Pi) {
        match crate::broker::start(state.client.clone(), &spec, &execution, &workspace).await {
            Ok(broker) => broker,
            Err(error) => {
                return fail_execution(
                    &state,
                    &execution,
                    HarnessError::new(
                        HarnessErrorKind::InvalidConfig,
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
    let materialized_skills = crate::skills::materialize(
        &spec,
        &execution.skills,
        &state.profile_skill_slugs,
        &context.skills,
        &workspace,
    )
    .await;
    if !materialized_skills.is_empty() {
        append_event(
            &state,
            &execution,
            ExecutionEventKind::Status,
            LogLevel::Info,
            &format!("materialized {} skill(s)", materialized_skills.len()),
            serde_json::json!({ "skills": materialized_skills }),
        )
        .await;
    }

    let (event_tx, mut event_rx) = mpsc::channel::<HarnessEvent>(256);
    let cancel = tokio_util::sync::CancellationToken::new();
    state
        .active
        .lock()
        .await
        .insert(execution_id, cancel.clone());

    state
        .client
        .running(execution_id, execution.session_ref.as_deref())
        .await?;
    append_event(
        &state,
        &execution,
        ExecutionEventKind::Status,
        LogLevel::Info,
        "execution dispatched to harness",
        serde_json::json!({ "harness": harness_kind.as_str(), "worker": state.worker_id }),
    )
    .await;

    // Persist harness events as they arrive.
    let event_state = state.clone();
    let event_execution = execution.clone();
    let event_task = tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let (kind, level, message, data) = map_harness_event(event);
            append_event(&event_state, &event_execution, kind, level, &message, data).await;
        }
    });

    // Deliver cancellation requests from the control plane.
    let watcher_client = state.client.clone();
    let watcher_cancel = cancel.clone();
    let done = Arc::new(AtomicBool::new(false));
    let watcher_done = done.clone();
    let watcher = tokio::spawn(async move {
        while !watcher_done.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            match watcher_client.cancel_requested(execution_id).await {
                Ok(true) => {
                    tracing::info!(%execution_id, "cancellation requested; cancelling harness");
                    watcher_cancel.cancel();
                    break;
                }
                Ok(false) => {}
                Err(err) => tracing::warn!(error = %err, "cancel poll failed"),
            }
        }
    });

    // Add the MCP bridge details to the harness config. This stays in memory only; the
    // execution-scoped token is invalid as soon as the broker is dropped.
    let mut harness_config = spec.harness.config.clone();
    if let (Some(handle), HarnessKind::OpenCode | HarnessKind::Pi) = (&broker, harness_kind) {
        if !harness_config.is_object() {
            harness_config = serde_json::json!({});
        }
        harness_config["orbitMcp"] = serde_json::json!({
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
        harness_config,
        session: SessionRequest {
            session_ref: execution.session_ref.clone(),
            mode: spec.harness.execution_mode,
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
            let mut outcome = ExecutionOutcome::failed(HarnessError::cancelled());
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
                let mut outcome = ExecutionOutcome::failed(HarnessError::timeout(timeout_seconds));
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
        .client
        .complete(
            execution_id,
            &crate::client::CompleteRequest {
                status: status.as_str(),
                output: outcome.output.clone(),
                error: error_json,
                usage: serde_json::to_value(&outcome.usage).ok(),
                session_ref: outcome.session_ref.as_deref(),
            },
        )
        .await?;
    if updated {
        tracing::info!(
            %execution_id,
            status = status.as_str(),
            duration_ms = started.elapsed().as_millis() as u64,
            "execution finished"
        );
    } else {
        tracing::warn!(%execution_id, "execution was already terminal; result discarded");
    }
    Ok(())
}

fn outcome_from_error(error: HarnessError) -> ExecutionOutcome {
    let status = match error.kind {
        HarnessErrorKind::Cancelled => ExecutionStatus::Cancelled,
        HarnessErrorKind::Timeout => ExecutionStatus::TimedOut,
        _ => ExecutionStatus::Failed,
    };
    let mut outcome = ExecutionOutcome::failed(error);
    outcome.status = status;
    outcome
}

pub(crate) async fn fail_execution(
    state: &WorkerState,
    execution: &ExecutionRecord,
    error: HarnessError,
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
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "execution failed".into()),
        serde_json::json!({ "error": outcome.error }),
    )
    .await;
    let error_json = outcome
        .error
        .as_ref()
        .and_then(|error| serde_json::to_value(error).ok());
    let _ = state
        .client
        .complete(
            execution.id,
            &crate::client::CompleteRequest {
                status: outcome.status.as_str(),
                output: None,
                error: error_json,
                usage: None,
                session_ref: None,
            },
        )
        .await;
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
    if let Err(error) = state
        .client
        .event(
            execution.id,
            &crate::client::EventRequest {
                kind: kind.as_str(),
                level: level.as_str(),
                message,
                data,
            },
        )
        .await
    {
        tracing::error!(%error, "failed to persist execution event");
    }
}

fn map_harness_event(
    event: HarnessEvent,
) -> (ExecutionEventKind, LogLevel, String, serde_json::Value) {
    let kind = match event.kind {
        HarnessEventKind::Started | HarnessEventKind::Status => ExecutionEventKind::Status,
        HarnessEventKind::Log => ExecutionEventKind::Log,
        HarnessEventKind::ToolCall => ExecutionEventKind::ToolCall,
        HarnessEventKind::OutputDelta => ExecutionEventKind::Output,
        HarnessEventKind::Usage => ExecutionEventKind::Metric,
        HarnessEventKind::Error => ExecutionEventKind::Error,
    };
    (kind, event.level, event.message, event.data)
}
