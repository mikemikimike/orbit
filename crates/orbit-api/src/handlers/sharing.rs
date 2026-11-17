//! Generic visibility sharing for shareable resources.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use orbit_domain::ValidationIssue;
use orbit_security::{AuthContext, Permission};

use crate::audit;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShareRequest {
    pub visibility: String,
    #[serde(default)]
    pub owner_team_id: Option<Uuid>,
}

#[utoipa::path(patch, path = "/api/v1/sharing/{resource}/{resource_id}", tag = "sharing", request_body = ShareRequest, responses((status = 204)))]
pub async fn share(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((resource, resource_id)): Path<(String, Uuid)>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<ShareRequest>,
) -> Result<StatusCode, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    state.require_feature("resource_permissions")?;
    if !matches!(body.visibility.as_str(), "private" | "team" | "org") {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "visibility",
            "must be private, team or org",
        )]));
    }
    if body.visibility == "team" && body.owner_team_id.is_none() {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "ownerTeamId",
            "ownerTeamId is required for team visibility",
        )]));
    }
    let (owner_user, owner_team, visibility) = state
        .store
        .resource_ownership(auth.org_id, &resource, resource_id)
        .await?;
    let access = crate::access::AccessScope::load(&state, &auth).await?;
    access.require_manage(&resource, owner_user, owner_team, &visibility)?;
    if let Some(team_id) = body.owner_team_id {
        state.store.get_access_team(auth.org_id, team_id).await?;
    }
    state
        .store
        .set_resource_visibility(
            auth.org_id,
            &resource,
            resource_id,
            &body.visibility,
            body.owner_team_id,
        )
        .await?;
    audit::record(
        &state,
        &auth,
        "resource.shared",
        Some(&resource),
        Some(resource_id),
        serde_json::json!({
            "visibility": body.visibility,
            "ownerTeamId": body.owner_team_id,
        }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/sharing/{resource}/{resource_id}", tag = "sharing", responses((status = 200, body = ShareRequest)))]
pub async fn get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path((resource, resource_id)): Path<(String, Uuid)>,
) -> Result<Json<ShareRequest>, ApiError> {
    auth.require(Permission::SettingsRead)?;
    let (owner_user, owner_team, visibility) = state
        .store
        .resource_ownership(auth.org_id, &resource, resource_id)
        .await?;
    let access = crate::access::AccessScope::load(&state, &auth).await?;
    access.require_view(&resource, owner_user, owner_team, &visibility)?;
    Ok(Json(ShareRequest {
        visibility,
        owner_team_id: owner_team,
    }))
}
