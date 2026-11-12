//! Agent Team work queue API.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use orbit_domain::ValidationIssue;
use orbit_security::{AuthContext, Permission};
use orbit_store::team_queue::TeamQueueItem;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueItemInput {
    pub external_key: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnqueueRequest {
    pub items: Vec<QueueItemInput>,
}

pub async fn enqueue(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(team_id): Path<Uuid>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<EnqueueRequest>,
) -> Result<(StatusCode, Json<Vec<TeamQueueItem>>), ApiError> {
    auth.require(Permission::ExecutionsCreate)?;
    let _team = state.store.get_agent_team(auth.org_id, team_id).await?;
    if body.items.is_empty() {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "items",
            "at least one queue item is required",
        )]));
    }
    let mut items = Vec::new();
    for item in body.items {
        let key = item.external_key.trim();
        if key.is_empty() {
            return Err(ApiError::validation(vec![ValidationIssue::new(
                "externalKey",
                "externalKey is required",
            )]));
        }
        items.push(
            state
                .store
                .enqueue_team_item(
                    auth.org_id,
                    team_id,
                    key,
                    item.repository
                        .as_deref()
                        .map(str::trim)
                        .filter(|v| !v.is_empty()),
                    &item.payload,
                )
                .await?,
        );
    }
    Ok((StatusCode::CREATED, Json(items)))
}

pub async fn list(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<Vec<TeamQueueItem>>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    let _team = state.store.get_agent_team(auth.org_id, team_id).await?;
    Ok(Json(
        state.store.list_team_queue(auth.org_id, team_id).await?,
    ))
}

pub async fn run_item(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((team_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<(StatusCode, Json<crate::handlers::teams::TeamRunDetail>), ApiError> {
    auth.require(Permission::ExecutionsCreate)?;
    let team = state.store.get_agent_team(auth.org_id, team_id).await?;
    let item = state
        .store
        .get_team_queue_item(auth.org_id, item_id)
        .await?;
    if item.team_id != team_id {
        return Err(ApiError::not_found("queue item"));
    }
    if item.status == "running" {
        return Err(ApiError::conflict("queue item is already running"));
    }
    let definition = crate::handlers::teams::parse_definition(&team)?;
    let task = item
        .payload
        .get("task")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| definition.instructions.clone());
    let mut payload = item.payload.clone();
    if !payload.is_object() {
        payload = serde_json::json!({});
    }
    if let Some(repository) = &item.repository {
        payload["repository"] = serde_json::json!(repository);
    }
    if payload
        .get("repository")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        if let Some(repository) = &definition.repository {
            payload["repository"] = serde_json::json!(repository);
        }
    }
    let input = serde_json::json!({
        "task": task,
        "instructions": definition.instructions,
        "payload": payload,
        "definition": definition,
        "autoApprove": false,
        "queueItemId": item.id,
        "externalKey": item.external_key,
    });
    let run = state
        .store
        .create_team_run(auth.org_id, team.id, &input, Some(auth.user_id))
        .await?;
    let step = crate::handlers::teams::start_step(&state, &run, &definition, 0, None).await?;
    if definition.auto_launch {
        if let (Some(profile_id), Some(execution_id)) =
            (definition.worker_profile_id, step.execution_id)
        {
            crate::handlers::worker_launches::launch_profile_internal(
                &state,
                &auth,
                profile_id,
                Some(execution_id),
            )
            .await?;
        }
    }
    state
        .store
        .set_team_queue_run(auth.org_id, item.id, run.id, "running")
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(crate::handlers::teams::TeamRunDetail {
            run,
            steps: vec![step],
        }),
    ))
}
