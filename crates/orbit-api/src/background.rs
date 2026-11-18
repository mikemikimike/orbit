//! Control-plane maintenance loops: scheduled triggers and orphaned-execution reaping.
//!
//! These used to run inside the worker, which required database access. They are
//! control-plane responsibilities, so they now run in `orbit-api` and keep workers
//! token-only.

use std::time::Duration;

use chrono::Utc;
use orbit_contracts::{subjects, OrbitEvent};
use orbit_domain::{AgentStatus, ExecutionStatus, LogLevel, TriggerSource};
use orbit_store::Trigger;

use crate::state::AppState;

pub fn spawn(state: AppState) {
    // Offline license: re-read the mounted file so airgapped renewals apply without a
    // restart (Secret volume updates propagate within ~60s).
    let license_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            license_state.license.reload();
        }
    });

    let output_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            crate::handlers::outputs::deliver_pending(&output_state).await;
        }
    });

    let scheduler_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(15));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = scheduler_tick(&scheduler_state).await {
                tracing::error!(%error, "scheduler tick failed");
            }
        }
    });

    let reaper_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(15));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            reap(&reaper_state).await;
        }
    });

    let team_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(2));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match team_state.store.list_running_team_runs().await {
                Ok(runs) => {
                    for run in runs {
                        if let Err(error) = crate::handlers::teams::advance(&team_state, run).await
                        {
                            tracing::warn!(%error, "team run advance failed");
                        }
                    }
                }
                Err(error) => tracing::warn!(%error, "team run polling failed"),
            }
        }
    });

    let dispatcher_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(15));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = crate::dispatcher::dispatch_tick(&dispatcher_state).await {
                tracing::error!(%error, "work source dispatcher tick failed");
            }
        }
    });

    let tag_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(error) = reconcile_worker_tags(&tag_state).await {
                tracing::warn!(%error, "worker tag reconciliation failed");
            }
        }
    });

    let retention_state = state.clone();
    tokio::spawn(async move {
        // Run shortly after start, then daily.
        let mut ticker = tokio::time::interval(Duration::from_secs(6 * 3600));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            archive_old_runs(&retention_state).await;
        }
    });

    let pruner_state = state.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(3600));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match pruner_state.store.prune_expired_memory().await {
                Ok(n) if n > 0 => tracing::info!(count = n, "pruned expired memory entries"),
                Ok(_) => {}
                Err(error) => tracing::warn!(%error, "memory pruning failed"),
            }
        }
    });
}

/// Archive terminal executions, team runs and worker launches older than the configured
/// retention window. History is kept; the default list simply stops showing old runs.
async fn archive_old_runs(state: &AppState) {
    let Some(days) = state.archive_after_days else {
        return;
    };
    let cutoff = Utc::now() - chrono::Duration::days(days as i64);
    match state.store.archive_executions_before(cutoff).await {
        Ok(0) => {}
        Ok(count) => tracing::info!(count, days, "archived old executions"),
        Err(error) => tracing::warn!(%error, "execution archive failed"),
    }
    match state.store.archive_team_runs_before(cutoff).await {
        Ok(0) => {}
        Ok(count) => tracing::info!(count, days, "archived old team runs"),
        Err(error) => tracing::warn!(%error, "team run archive failed"),
    }
    match state.store.archive_worker_launches_before(cutoff).await {
        Ok(0) => {}
        Ok(count) => tracing::info!(count, days, "archived old worker launches"),
        Err(error) => tracing::warn!(%error, "worker launch archive failed"),
    }
}

async fn scheduler_tick(state: &AppState) -> anyhow::Result<()> {
    for trigger in state.store.claim_due_triggers(10).await? {
        if let Err(error) = fire_trigger(state, trigger).await {
            tracing::warn!(%error, "scheduled trigger failed");
        }
    }
    Ok(())
}

async fn fire_trigger(state: &AppState, trigger: Trigger) -> anyhow::Result<()> {
    let result = fire_trigger_inner(state, trigger.clone()).await;
    match &result {
        Ok(()) => {
            let _ = state.store.record_trigger_success(trigger.id).await;
        }
        Err(error) => {
            tracing::warn!(%error, trigger_id = %trigger.id, "scheduled trigger failed");
            if let Ok(disabled) = state.store.record_trigger_failure(trigger.id).await {
                if disabled {
                    tracing::warn!(trigger_id = %trigger.id, "trigger auto-disabled after consecutive failures");
                }
            }
        }
    }
    result
}

async fn fire_trigger_inner(state: &AppState, trigger: Trigger) -> anyhow::Result<()> {
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
    // A schedule may carry its own task; otherwise the agent gets a generic prompt.
    let task = trigger
        .config
        .get("task")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|task| !task.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "Scheduled run for agent '{}'. Perform your configured monitoring task.",
                agent.name
            )
        });
    let input = serde_json::json!({
        "task": task,
        "payload": { "triggerId": trigger.id, "firedAt": fired_at },
        "provenance": {
            "source": "schedule",
            "trusted": false,
            "subject": trigger.id,
            "receivedAt": fired_at,
        },
        "agentVersion": version.version,
    });

    let harness_kind = match trigger
        .harness_kind
        .as_deref()
        .and_then(|value| value.parse::<orbit_domain::HarnessKind>().ok())
        .or_else(|| spec.harness.as_ref().map(|harness| harness.kind))
    {
        Some(kind) => kind,
        None => match state.default_harness().await {
            Some(kind) => kind,
            None => {
                tracing::warn!(trigger_id = %trigger.id, "scheduled trigger skipped: no harness configured");
                return Ok(());
            }
        },
    };
    let placement = spec
        .harness
        .as_ref()
        .map(|harness| harness.placement.as_str())
        .unwrap_or("auto");
    let idempotency_key = format!("trigger:{}:{}", trigger.id, fired_at.timestamp_millis());
    let execution = state
        .store
        .create_execution(orbit_store::executions::NewExecution {
            org_id: trigger.org_id,
            agent_id: agent.id,
            agent_version_id: version.id,
            agent_name: &agent.name,
            harness_kind,
            trigger_source: TriggerSource::Schedule,
            placement,
            trigger_id: Some(trigger.id),
            input,
            skills: &[],
            timeout_seconds: spec.resources.timeout_seconds,
            requested_by: None,
            idempotency_key: Some(idempotency_key.as_str()),
            team_run_id: None,
            team_step_index: None,
        })
        .await?;

    let event = OrbitEvent::new(
        subjects::EXECUTION_REQUESTED,
        "orbit-api-scheduler",
        serde_json::json!({
            "executionId": execution.id,
            "agentId": agent.id,
            "triggerId": trigger.id,
            "harness": harness_kind.as_str(),
        }),
    )
    .with_org(trigger.org_id)
    .with_idempotency(format!("exec:{}", execution.id));
    if let Err(error) = state.bus.publish(event).await {
        tracing::warn!(%error, "scheduled execution publish failed; polling will pick it up");
    }
    tracing::info!(
        execution_id = %execution.id,
        agent_id = %agent.id,
        status = ExecutionStatus::Queued.as_str(),
        "scheduled execution queued"
    );
    Ok(())
}

async fn reap(state: &AppState) {
    match state.store.reap_orphaned_executions(90).await {
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
                    "orbit-api-reaper",
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

/// Relaunch platform-follow worker profiles when ORBIT_IMAGE_TAG changes. The old token
/// is revoked first (no new claims), active executions drain, then the launcher stops and
/// replaces the worker.
async fn reconcile_worker_tags(state: &AppState) -> anyhow::Result<()> {
    if state.launcher_url.is_none() {
        return Ok(());
    }
    let tag = crate::images::image_tag();
    let profiles = state.store.list_platform_follow_profiles().await?;
    for profile in profiles {
        let Some(user_id) = profile.owner_user_id else {
            continue;
        };
        let desired = format!("{}:{}", profile.image, tag);
        let launches = state
            .store
            .list_worker_launches(profile.org_id, None)
            .await?;
        for launch in launches.into_iter().filter(|launch| {
            launch.profile_id == profile.id
                && matches!(launch.status.as_str(), "pending" | "starting" | "running")
                && launch.image != desired
        }) {
            let auth = orbit_security::AuthContext {
                user_id,
                org_id: profile.org_id,
                email: "system@orbit.local".into(),
                display_name: "ORBIT auto-update".into(),
                role: orbit_domain::UserRole::Owner,
                method: orbit_security::AuthMethod::ApiToken,
                scopes: Vec::new(),
                token_id: None,
            };
            if let Some(token_id) = launch.worker_token_id {
                let _ = state
                    .store
                    .revoke_worker_token(profile.org_id, token_id)
                    .await;
            }
            if let Some(worker_id) = &launch.worker_id {
                if state
                    .store
                    .has_active_executions_for_worker(profile.org_id, worker_id)
                    .await?
                {
                    tracing::info!(profile = %profile.name, %worker_id, "worker update waiting for drain");
                    continue;
                }
            }
            let old_image = launch.image.clone();
            match crate::handlers::worker_launches::stop_launch_internal(state, &auth, launch).await
            {
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(profile = %profile.name, %error, "could not stop old worker");
                    continue;
                }
            }
            match crate::handlers::worker_launches::launch_profile_internal(
                state, &auth, profile.id, None,
            )
            .await
            {
                Ok(new_launch) => {
                    tracing::info!(
                        profile = %profile.name,
                        old = %old_image,
                        new = %new_launch.image,
                        "worker auto-relaunched for platform tag"
                    );
                    crate::audit::record(
                        state,
                        &auth,
                        "worker.auto_relaunched",
                        Some("worker_profile"),
                        Some(profile.id),
                        serde_json::json!({
                            "oldImage": old_image,
                            "newImage": new_launch.image,
                            "launchId": new_launch.id,
                        }),
                    )
                    .await;
                }
                Err(error) => {
                    tracing::error!(profile = %profile.name, %error, "worker auto-relaunch failed");
                }
            }
        }
    }
    Ok(())
}
