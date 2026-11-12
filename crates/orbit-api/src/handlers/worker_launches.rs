//! Launch/stop worker instances through the optional `orb-launcher` service.
//!
//! The API stores launch intent and status, but never receives Docker socket or
//! Kubernetes privileges. The launcher exposes a small internal HTTP surface.

use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use orbit_security::{AuthContext, Permission};
use orbit_store::WorkerLaunch;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LauncherRequest {
    launch_id: Uuid,
    profile_id: Uuid,
    name: String,
    harness_kind: String,
    target: String,
    execution_driver: String,
    image: String,
    image_tag: String,
    auth_secret_name: Option<String>,
    env: serde_json::Value,
    resources: serde_json::Value,
    api_url: String,
    worker_token: String,
    worker_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    single_execution_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LauncherStatus {
    status: String,
    #[serde(default)]
    runtime_id: Option<String>,
    #[serde(default)]
    worker_id: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LaunchLogs {
    pub logs: String,
}

fn launcher(state: &AppState) -> Result<(&str, &str), ApiError> {
    match (&state.launcher_url, &state.launcher_token) {
        (Some(url), Some(token)) => Ok((url.trim_end_matches('/'), token)),
        _ => Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "launcher_unavailable",
            "no worker launcher is configured for this deployment",
        )),
    }
}

async fn call_launcher<T: for<'de> Deserialize<'de>>(
    state: &AppState,
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<T, ApiError> {
    let (url, token) = launcher(state)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut request = client
        .request(method, format!("{url}{path}"))
        .bearer_auth(token);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.map_err(|error| {
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "launcher_error",
            format!("worker launcher request failed: {error}"),
        )
    })?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "launcher_error",
            format!("worker launcher returned {status}: {body}"),
        ));
    }
    response.json().await.map_err(|error| {
        ApiError::new(StatusCode::BAD_GATEWAY, "launcher_error", error.to_string())
    })
}

async fn refresh(state: &AppState, launch: WorkerLaunch) -> WorkerLaunch {
    if !matches!(launch.status.as_str(), "pending" | "starting" | "running") {
        return launch;
    }
    let (url, token) = match (&state.launcher_url, &state.launcher_token) {
        (Some(url), Some(token)) => (url.trim_end_matches('/'), token),
        _ => return launch,
    };
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
    {
        Ok(client) => client,
        Err(_) => return launch,
    };
    let response = client
        .get(format!(
            "{url}/launches/{}?target={}",
            launch.id, launch.target
        ))
        .bearer_auth(token)
        .send()
        .await;
    let Ok(response) = response else {
        return launch;
    };
    let Ok(status) = response.json::<LauncherStatus>().await else {
        return launch;
    };
    let stopped = matches!(status.status.as_str(), "stopped" | "failed");
    state
        .store
        .update_worker_launch(
            launch.org_id,
            launch.id,
            &status.status,
            status.runtime_id.as_deref(),
            status.worker_id.as_deref(),
            status.error.as_deref(),
            status.status == "running",
            stopped,
        )
        .await
        .unwrap_or(launch)
}

#[utoipa::path(get, path = "/api/v1/worker-launches", tag = "workers", responses((status = 200, body = [WorkerLaunch])))]
pub async fn list_launches(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<WorkerLaunch>>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    let launches = state.store.list_worker_launches(auth.org_id).await?;
    let mut refreshed = Vec::with_capacity(launches.len());
    for launch in launches {
        refreshed.push(refresh(&state, launch).await);
    }
    Ok(Json(refreshed))
}

#[utoipa::path(get, path = "/api/v1/worker-launches/{launch_id}", tag = "workers", responses((status = 200, body = WorkerLaunch)))]
pub async fn get_launch(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(launch_id): Path<Uuid>,
) -> Result<Json<WorkerLaunch>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    let launch = state
        .store
        .get_worker_launch(auth.org_id, launch_id)
        .await?;
    Ok(Json(refresh(&state, launch).await))
}

#[utoipa::path(post, path = "/api/v1/worker-profiles/{profile_id}/launch", tag = "workers", responses((status = 201, body = WorkerLaunch)))]
pub(crate) async fn launch_profile_internal(
    state: &AppState,
    auth: &AuthContext,
    profile_id: Uuid,
    single_execution_id: Option<Uuid>,
) -> Result<WorkerLaunch, ApiError> {
    launcher(state)?;
    let profile = state
        .store
        .get_worker_profile(auth.org_id, profile_id)
        .await?;
    if !profile.enabled {
        return Err(ApiError::conflict(
            "enable the worker profile before launching it",
        ));
    }
    // Reuse an existing live launch for this profile instead of creating a new worker
    // for every TeamRun. Single-execution launches are always isolated.
    if single_execution_id.is_none() {
        if let Some(existing) = state
            .store
            .list_worker_launches(auth.org_id)
            .await?
            .into_iter()
            .find(|launch| {
                launch.profile_id == profile.id
                    && matches!(launch.status.as_str(), "pending" | "starting" | "running")
            })
        {
            return Ok(existing);
        }
    }
    let issued =
        crate::handlers::worker_profiles::issue_profile_token(state, auth.org_id, &profile, false)
            .await?;
    let image = format!("{}:{}", profile.image, profile.image_tag);
    let launch = state
        .store
        .create_worker_launch(
            auth.org_id,
            profile.id,
            &profile.name,
            &profile.target,
            &image,
            Some(issued.token_id),
            Some(auth.user_id),
        )
        .await?;
    let worker_id = format!("worker-{}", &launch.id.to_string()[..8]);
    if let Some(execution_id) = single_execution_id {
        state
            .store
            .claim_execution(execution_id, &worker_id)
            .await?
            .ok_or_else(|| ApiError::conflict("execution is no longer queued for this launch"))?;
    }
    let request = LauncherRequest {
        launch_id: launch.id,
        profile_id: profile.id,
        name: profile.name.clone(),
        harness_kind: profile.harness_kind.clone(),
        target: profile.target.clone(),
        execution_driver: profile.execution_driver.clone(),
        image: profile.image.clone(),
        image_tag: profile.image_tag.clone(),
        auth_secret_name: profile.auth_secret_name.clone(),
        env: profile.env.clone(),
        resources: profile.resources.clone(),
        api_url: state.worker_api_url.clone(),
        worker_token: issued.token,
        worker_id: worker_id.clone(),
        single_execution_id,
    };
    let result: LauncherStatus = match call_launcher(
        state,
        reqwest::Method::POST,
        "/launches",
        Some(serde_json::to_value(&request).unwrap_or_default()),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            let _ = state
                .store
                .revoke_worker_token(auth.org_id, issued.token_id)
                .await;
            let _ = state
                .store
                .update_worker_launch(
                    auth.org_id,
                    launch.id,
                    "failed",
                    None,
                    None,
                    Some(&error.message),
                    false,
                    true,
                )
                .await;
            if let Some(execution_id) = single_execution_id {
                let _ = state
                    .store
                    .finish_execution(
                        execution_id,
                        orbit_domain::ExecutionStatus::Failed,
                        None,
                        Some(&serde_json::json!({
                            "code": "launch_failed",
                            "message": error.message
                        })),
                        None,
                        None,
                    )
                    .await;
            }
            return Err(error);
        }
    };
    state
        .store
        .update_worker_launch(
            auth.org_id,
            launch.id,
            &result.status,
            result.runtime_id.as_deref(),
            Some(result.worker_id.as_deref().unwrap_or(&worker_id)),
            result.error.as_deref(),
            true,
            false,
        )
        .await
        .map_err(Into::into)
}

#[utoipa::path(post, path = "/api/v1/worker-profiles/{profile_id}/launch", tag = "workers", responses((status = 201, body = WorkerLaunch)))]
pub async fn launch_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
) -> Result<(StatusCode, Json<WorkerLaunch>), ApiError> {
    auth.require(Permission::SettingsWrite)?;
    Ok((
        StatusCode::CREATED,
        Json(launch_profile_internal(&state, &auth, profile_id, None).await?),
    ))
}

#[utoipa::path(post, path = "/api/v1/worker-launches/{launch_id}/stop", tag = "workers", responses((status = 200, body = WorkerLaunch)))]
pub async fn stop_launch(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(launch_id): Path<Uuid>,
) -> Result<Json<WorkerLaunch>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let launch = state
        .store
        .get_worker_launch(auth.org_id, launch_id)
        .await?;
    let _: serde_json::Value = call_launcher(
        &state,
        reqwest::Method::POST,
        &format!("/launches/{launch_id}/stop?target={}", launch.target),
        Some(serde_json::json!({})),
    )
    .await?;
    if let Some(token_id) = launch.worker_token_id {
        let _ = state.store.revoke_worker_token(auth.org_id, token_id).await;
    }
    Ok(Json(
        state
            .store
            .update_worker_launch(
                auth.org_id,
                launch_id,
                "stopped",
                None,
                None,
                None,
                false,
                true,
            )
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/worker-launches/{launch_id}/logs", tag = "workers", responses((status = 200, body = LaunchLogs)))]
pub async fn launch_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(launch_id): Path<Uuid>,
) -> Result<Json<LaunchLogs>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    let launch = state
        .store
        .get_worker_launch(auth.org_id, launch_id)
        .await?;
    let logs: LaunchLogs = call_launcher(
        &state,
        reqwest::Method::GET,
        &format!("/launches/{launch_id}/logs?target={}", launch.target),
        None,
    )
    .await?;
    Ok(Json(logs))
}
