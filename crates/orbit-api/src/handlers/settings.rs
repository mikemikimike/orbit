//! Platform settings (OIDC, etc.) manageable from the UI.

use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};

use orbit_security::{AuthContext, Permission};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcSettings {
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub redirect_uri: String,
    #[serde(default)]
    pub scopes: String,
    #[serde(default)]
    pub audience: String,
    #[serde(default)]
    pub jwks_url: String,
    #[serde(default)]
    pub auto_provision: bool,
    #[serde(default)]
    pub default_role: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOidcRequest {
    pub settings: OidcSettings,
}

pub async fn get_oidc(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<OidcSettings>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let value = state
        .store
        .get_platform_setting("oidc")
        .await?
        .unwrap_or_else(|| serde_json::json!({}));
    let settings: OidcSettings = serde_json::from_value(value).unwrap_or(OidcSettings {
        issuer: String::new(),
        client_id: String::new(),
        redirect_uri: String::new(),
        scopes: "openid profile email".into(),
        audience: String::new(),
        jwks_url: String::new(),
        auto_provision: true,
        default_role: "viewer".into(),
    });
    Ok(Json(settings))
}

pub async fn update_oidc(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<UpdateOidcRequest>,
) -> Result<Json<OidcSettings>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let value =
        serde_json::to_value(&body.settings).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let stored = state
        .store
        .set_platform_setting("oidc", &value, Some(auth.user_id))
        .await?;
    let settings: OidcSettings = serde_json::from_value(stored).unwrap_or(body.settings);
    Ok(Json(settings))
}
