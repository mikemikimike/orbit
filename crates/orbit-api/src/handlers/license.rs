//! License status and usage overview.

use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Serialize;

use orbit_security::{AuthContext, LicenseStatus, Permission};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseUsage {
    pub seats: i64,
    pub agents: i64,
    pub workers: i64,
    pub executions_today: i64,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseOverview {
    pub license: LicenseStatus,
    pub usage: LicenseUsage,
    pub warnings: Vec<String>,
}

#[utoipa::path(get, path = "/api/v1/license", tag = "license", responses((status = 200, body = LicenseOverview)))]
pub async fn overview(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<LicenseOverview>, ApiError> {
    auth.require(Permission::SettingsRead)?;
    let since = chrono::Utc::now() - chrono::Duration::hours(24);
    let usage = LicenseUsage {
        seats: state.store.count_active_users(auth.org_id).await?,
        agents: state.store.count_agents(auth.org_id).await?,
        workers: state
            .store
            .count_enabled_worker_profiles(auth.org_id)
            .await?,
        executions_today: state
            .store
            .count_all_executions_since(auth.org_id, since)
            .await?,
    };
    let license = state.license_status();
    let mut warnings = Vec::new();
    for (key, current) in [
        ("seats", usage.seats),
        ("agents", usage.agents),
        ("workers", usage.workers),
        ("executions_per_day", usage.executions_today),
    ] {
        if let Some(limit) = license.limit(key) {
            if limit > 0 && current * 100 >= limit as i64 * 80 {
                warnings.push(format!("{key}: {current}/{limit} used"));
            }
        }
    }
    Ok(Json(LicenseOverview {
        license,
        usage,
        warnings,
    }))
}
