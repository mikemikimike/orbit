//! Worker heartbeat.
//!
//! Orphaned-execution reaping is a control-plane responsibility and runs in `orbit-api`
//! against the same heartbeat table.

use std::time::Duration;

use crate::runner::WorkerState;

pub fn spawn(state: WorkerState, heartbeat_interval: Duration) {
    tokio::spawn(async move {
        beat(&state).await;
        let mut ticker = tokio::time::interval(heartbeat_interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            beat(&state).await;
        }
    });
}

async fn beat(state: &WorkerState) {
    let hostname = std::env::var("HOSTNAME").ok();
    if let Err(error) = state
        .client
        .heartbeat(&crate::client::HeartbeatRequest {
            worker_id: &state.worker_id,
            harnesses: &state.harnesses,
            target: &state.target,
            hostname: hostname.as_deref(),
        })
        .await
    {
        tracing::warn!(%error, "failed to write worker heartbeat");
    }
}
