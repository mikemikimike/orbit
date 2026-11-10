use std::time::Duration;

use chrono::Utc;
use orbit_contracts::{subjects, OrbitEvent};
use orbit_domain::{AgentStatus, ExecutionStatus, TriggerSource};
use orbit_store::Trigger;

use crate::runner::WorkerState;

/// Scheduled trigger loop. Phase 0 runs a single scheduler per worker deployment; HA
/// scheduling is a Phase 2/3 item (a Postgres advisory lock is the planned mechanism).
pub async fn run(state: WorkerState) {
    let mut ticker = tokio::time::interval(Duration::from_secs(15));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        if let Err(err) = tick(&state).await {
            tracing::error!(error = %err, "scheduler tick failed");
        }
    }
}

async fn tick(state: &WorkerState) -> anyhow::Result<()> {
    let due = state.store.claim_due_triggers(10).await?;
    for trigger in due {
        if let Err(err) = fire_trigger(state, trigger).await {
            tracing::warn!(error = %err, "scheduled trigger failed");
        }
    }
    Ok(())
}

async fn fire_trigger(state: &WorkerState, trigger: Trigger) -> anyhow::Result<()> {
    // Claiming nulls next_run_at; compute the following occurrence now.
    let next = trigger
        .config
        .get("cron")
        .and_then(|value| value.as_str())
        .and_then(|cron| cron.parse::<croner::Cron>().ok())
        .and_then(|cron| cron.find_next_occurrence(&Utc::now(), false).ok());
    state.store.reschedule_trigger(trigger.id, next).await?;

    let agent = state
        .store
        .get_agent(trigger.org_id, trigger.agent_id)
        .await?;
    if AgentStatus::parse(&agent.status) != Some(AgentStatus::Active) {
        tracing::debug!(agent_id = %agent.id, "scheduled trigger skipped: agent is not active");
        return Ok(());
    }
    let version = state
        .store
        .get_current_version(trigger.org_id, &agent)
        .await?;
    let spec: orbit_domain::AgentSpec = serde_json::from_value(version.spec.clone())?;

    let active = state
        .store
        .count_active_executions_for_agent(trigger.org_id, agent.id)
        .await?;
    if active >= spec.resources.max_concurrent_executions as i64 {
        tracing::info!(agent_id = %agent.id, "scheduled trigger skipped: concurrency limit");
        return Ok(());
    }

    let fired_at = trigger.last_fired_at.unwrap_or_else(Utc::now);
    let input = serde_json::json!({
        "task": format!("Scheduled run for agent '{}'. Perform your configured monitoring task.", agent.name),
        "payload": { "triggerId": trigger.id, "firedAt": fired_at },
        "provenance": {
            "source": "schedule",
            "trusted": false,
            "subject": trigger.id,
            "receivedAt": fired_at,
        },
        "agentVersion": version.version,
    });

    let idempotency_key = format!("trigger:{}:{}", trigger.id, fired_at.timestamp_millis());
    let execution = state
        .store
        .create_execution(orbit_store::executions::NewExecution {
            org_id: trigger.org_id,
            agent_id: agent.id,
            agent_version_id: version.id,
            agent_name: &agent.name,
            runtime_kind: spec.runtime.kind,
            trigger_source: TriggerSource::Schedule,
            placement: spec.runtime.placement.as_str(),
            trigger_id: Some(trigger.id),
            input,
            timeout_seconds: spec.resources.timeout_seconds,
            requested_by: None,
            idempotency_key: Some(idempotency_key.as_str()),
        })
        .await?;

    let event = OrbitEvent::new(
        subjects::EXECUTION_REQUESTED,
        "orbit-worker-scheduler",
        serde_json::json!({
            "executionId": execution.id,
            "agentId": agent.id,
            "triggerId": trigger.id,
            "runtime": spec.runtime.kind.as_str(),
        }),
    )
    .with_org(trigger.org_id)
    .with_idempotency(format!("exec:{}", execution.id));
    if let Err(err) = state.bus.publish(event).await {
        tracing::warn!(error = %err, "scheduled execution publish failed; polling will pick it up");
    }

    tracing::info!(
        execution_id = %execution.id,
        agent_id = %agent.id,
        status = ExecutionStatus::Queued.as_str(),
        "scheduled execution queued"
    );
    Ok(())
}
