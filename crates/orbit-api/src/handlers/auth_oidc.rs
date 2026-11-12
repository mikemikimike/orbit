//! Browser OIDC helpers: discovery plus Authorization Code + PKCE token exchange.
//!
//! The browser never sees the client secret. Dex and Keycloak both expose standard
//! discovery and token endpoints, so the flow is provider-neutral.

use std::time::Duration;

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcWebConfig {
    pub issuer: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub authorization_endpoint: String,
    pub end_session_endpoint: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcTokenRequest {
    pub code: String,
    pub code_verifier: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcTokenResponse {
    pub access_token: Option<String>,
    pub id_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: Option<String>,
}

fn oidc(state: &AppState) -> Result<&orbit_security::OidcConfig, ApiError> {
    state
        .auth
        .oidc
        .as_ref()
        .ok_or_else(|| ApiError::not_found("OIDC authentication"))
}

async fn discovery(issuer: &str) -> Result<Value, ApiError> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    );
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|error| ApiError::internal(error.to_string()))?
        .get(&url)
        .send()
        .await
        .map_err(|error| {
            ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "oidc_discovery_failed",
                format!("could not reach OIDC discovery: {error}"),
            )
        })?;
    if !response.status().is_success() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "oidc_discovery_failed",
            format!("OIDC discovery returned {}", response.status()),
        ));
    }
    response
        .json()
        .await
        .map_err(|error| ApiError::internal(format!("invalid OIDC discovery: {error}")))
}

pub async fn config(State(state): State<AppState>) -> Result<Json<OidcWebConfig>, ApiError> {
    let config = oidc(&state)?;
    let document = discovery(&config.issuer).await?;
    let authorization_endpoint = document
        .get("authorization_endpoint")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("OIDC discovery has no authorization_endpoint"))?
        .to_string();
    Ok(Json(OidcWebConfig {
        issuer: config.issuer.clone(),
        client_id: config.client_id.clone(),
        redirect_uri: config.redirect_uri.clone(),
        scopes: config.scopes.clone(),
        authorization_endpoint,
        end_session_endpoint: document
            .get("end_session_endpoint")
            .and_then(Value::as_str)
            .map(str::to_string),
    }))
}

pub async fn token(
    State(state): State<AppState>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<OidcTokenRequest>,
) -> Result<Json<OidcTokenResponse>, ApiError> {
    let config = oidc(&state)?;
    if body.code.trim().is_empty() || body.code_verifier.trim().is_empty() {
        return Err(ApiError::bad_request("code and codeVerifier are required"));
    }
    let document = discovery(&config.issuer).await?;
    let token_endpoint = document
        .get("token_endpoint")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::internal("OIDC discovery has no token_endpoint"))?;
    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", body.code),
        ("redirect_uri", config.redirect_uri.clone()),
        ("client_id", config.client_id.clone()),
        ("code_verifier", body.code_verifier),
    ];
    if let Some(secret) = &config.client_secret {
        form.push(("client_secret", secret.clone()));
    }
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| ApiError::internal(error.to_string()))?
        .post(token_endpoint)
        .form(&form)
        .send()
        .await
        .map_err(|error| {
            ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "oidc_token_failed",
                format!("token exchange failed: {error}"),
            )
        })?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
    if !status.is_success() {
        let message = value
            .get("error_description")
            .or_else(|| value.get("error"))
            .and_then(Value::as_str)
            .unwrap_or("OIDC token exchange failed");
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "oidc_token_failed",
            message,
        ));
    }
    Ok(Json(OidcTokenResponse {
        access_token: value
            .get("access_token")
            .and_then(Value::as_str)
            .map(str::to_string),
        id_token: value
            .get("id_token")
            .and_then(Value::as_str)
            .map(str::to_string),
        refresh_token: value
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(str::to_string),
        expires_in: value.get("expires_in").and_then(Value::as_u64),
        token_type: value
            .get("token_type")
            .and_then(Value::as_str)
            .map(str::to_string),
    }))
}
