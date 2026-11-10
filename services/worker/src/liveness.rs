//! Worker heartbeat and orphaned-execution recovery.
//!
//! A worker that crashes cannot update its executions. A periodic reaper marks those
//! executions failed once the worker heartbeat is older than the configured threshold,
//! and expires any pending approvals so the UI does not show dead requests forever.

use std::time::Duration;

use orbit_contracts::{subjects, OrbitEvent};
use orbit_domain::LogLevel;

use crate::runner::WorkerState;

pub fn spawn(state: WorkerState, heartbeat_interval: Duration, stale_after: Duration) {
    let heartbeat_state = state.clone();
    tokio::spawn(async move {
        beat(&heartbeat_state).await;
        let mut ticker = tokio::time::interval(heartbeat_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            beat(&heartbeat_state).await;
        }
    });

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(15));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match state
                .store
                .reap_orphaned_executions(stale_after.as_secs() as i64)
                .await
            {
                Ok(rows) if !rows.is_empty() => {
                    for execution in rows {
                        tracing::warn!(
                            execution_id = %execution.id,
                            worker_id = ?execution.worker_id,
                            "reaped orphaned execution"
                        );
                        let _ = state
                            .store
                            .append_execution_event(
                                execution.org_id,
                                execution.id,
                                "error",
                                LogLevel::Error,
                                "worker heartbeat expired; execution marked failed",
                                &orbit_store::workers::worker_lost_error(),
                            )
                            .await;
                        let event = OrbitEvent::new(
                            subjects::EXECUTION_UPDATED,
                            "orbit-worker-reaper",
                            serde_json::json!({
                                "executionId": execution.id,
                                "agentId": execution.agent_id,
                                "status": execution.status,
                            }),
                        )
                        .with_org(execution.org_id)
                        .with_idempotency(format!("reap:{}", execution.id));
                        let _ = state.bus.publish(event).await;
                    }
                }
                Ok(_) => {}
                Err(error) => tracing::warn!(%error, "execution reaper failed"),
            }
        }
    });
}

async fn beat(state: &WorkerState) {
    let hostname = std::env::var("HOSTNAME").ok();
    if let Err(error) = state
        .store
        .upsert_worker_heartbeat(
            &state.worker_id,
            &state.harnesses,
            &state.target,
            hostname.as_deref(),
        )
        .await
    {
        tracing::warn!(%error, "failed to write worker heartbeat");
    }
}
