//! Custom roles: named permission sets per organization (enterprise RBAC foundation).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use uuid::Uuid;

use orbit_domain::UserRole;
use orbit_security::{AuthContext, Permission};
use orbit_store::CustomRole;

use crate::audit;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoleRequest {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

fn allowed_permissions() -> std::collections::BTreeSet<&'static str> {
    Permission::all_for(UserRole::Owner).into_iter().collect()
}

fn validated(body: &RoleRequest) -> Result<(String, Vec<String>), ApiError> {
    let name = body.name.trim().to_string();
    let mut issues = Vec::new();
    if name.is_empty() {
        issues.push(orbit_domain::ValidationIssue::new(
            "name",
            "name is required",
        ));
    }
    if name.chars().count() > 80 {
        issues.push(orbit_domain::ValidationIssue::new(
            "name",
            "name must be <= 80 characters",
        ));
    }
    if name.parse::<UserRole>().is_ok() {
        issues.push(orbit_domain::ValidationIssue::new(
            "name",
            "built-in role names are reserved",
        ));
    }
    let allowed = allowed_permissions();
    let mut permissions: Vec<String> = body
        .permissions
        .iter()
        .map(|permission| permission.trim().to_string())
        .filter(|permission| !permission.is_empty())
        .collect();
    permissions.sort();
    permissions.dedup();
    for permission in &permissions {
        if permission != "*" && !allowed.contains(permission.as_str()) {
            issues.push(orbit_domain::ValidationIssue::new(
                "permissions",
                format!("unknown permission '{permission}'"),
            ));
        }
    }
    if permissions.is_empty() {
        issues.push(orbit_domain::ValidationIssue::new(
            "permissions",
            "at least one permission is required",
        ));
    }
    if !issues.is_empty() {
        return Err(ApiError::validation(issues));
    }
    Ok((name, permissions))
}

#[utoipa::path(get, path = "/api/v1/roles", tag = "roles", responses((status = 200, body = [CustomRole])))]
pub async fn list(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<CustomRole>>, ApiError> {
    auth.require(Permission::SettingsRead)?;
    Ok(Json(state.store.list_custom_roles(auth.org_id).await?))
}

#[utoipa::path(post, path = "/api/v1/roles", tag = "roles", request_body = RoleRequest, responses((status = 201, body = CustomRole)))]
pub async fn create(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<RoleRequest>,
) -> Result<(StatusCode, Json<CustomRole>), ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let (name, permissions) = validated(&body)?;
    let role = state
        .store
        .create_custom_role(auth.org_id, &name, &permissions)
        .await?;
    audit::record(
        &state,
        &auth,
        "role.created",
        Some("role"),
        Some(role.id),
        serde_json::json!({ "name": role.name, "permissions": role.permissions }),
    )
    .await;
    Ok((StatusCode::CREATED, Json(role)))
}

#[utoipa::path(patch, path = "/api/v1/roles/{role_id}", tag = "roles", request_body = RoleRequest, responses((status = 200, body = CustomRole)))]
pub async fn update(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(role_id): Path<Uuid>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<RoleRequest>,
) -> Result<Json<CustomRole>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let (name, permissions) = validated(&body)?;
    let role = state
        .store
        .update_custom_role(auth.org_id, role_id, &name, &permissions)
        .await?;
    audit::record(
        &state,
        &auth,
        "role.updated",
        Some("role"),
        Some(role.id),
        serde_json::json!({ "name": role.name, "permissions": role.permissions }),
    )
    .await;
    Ok(Json(role))
}

#[utoipa::path(delete, path = "/api/v1/roles/{role_id}", tag = "roles", responses((status = 204)))]
pub async fn delete(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(role_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let role = state.store.get_custom_role(auth.org_id, role_id).await?;
    state.store.delete_custom_role(auth.org_id, role_id).await?;
    audit::record(
        &state,
        &auth,
        "role.deleted",
        Some("role"),
        Some(role.id),
        serde_json::json!({ "name": role.name }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
