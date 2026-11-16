use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use orbit_domain::{HarnessKind, ValidationIssue};
use orbit_security::{issue_worker_token, AuthContext, Permission};
use orbit_store::WorkerProfile;

use crate::audit;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkerProfileRequest {
    pub name: String,
    pub harness_kind: String,
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default = "default_driver")]
    pub execution_driver: String,
    #[serde(default)]
    pub image: String,
    #[serde(default = "default_tag")]
    pub image_tag: String,
    #[serde(default)]
    pub agent_ids: Vec<Uuid>,
    #[serde(default)]
    pub skill_slugs: Vec<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub auth_secret_name: Option<String>,
    #[serde(default)]
    pub env: serde_json::Value,
    #[serde(default)]
    pub env_mappings: serde_json::Value,
    #[serde(default)]
    pub resources: serde_json::Value,
}

fn default_target() -> String {
    "docker".into()
}
fn default_driver() -> String {
    "process".into()
}
fn default_tag() -> String {
    "latest".into()
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerBootstrap {
    pub docker: String,
    pub compose: String,
    pub kubernetes: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkerTokenRequest {
    pub name: String,
    #[serde(default)]
    pub harnesses: Vec<String>,
    #[serde(default)]
    pub profile_id: Option<Uuid>,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuedWorkerToken {
    pub token: String,
    pub token_id: Uuid,
    pub profile_id: Option<Uuid>,
    pub harnesses: Vec<String>,
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkerTokenView {
    pub id: Uuid,
    pub name: String,
    pub profile_id: Option<Uuid>,
    pub harnesses: Vec<String>,
    pub token_prefix: String,
    pub last_used_at: Option<chrono::DateTime<chrono::Utc>>,
    pub revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub(crate) async fn issue_profile_token(
    state: &AppState,
    org_id: Uuid,
    profile: &WorkerProfile,
    rotate: bool,
) -> Result<IssuedWorkerToken, ApiError> {
    if rotate {
        state
            .store
            .revoke_profile_worker_tokens(org_id, profile.id)
            .await?;
    }
    let issued = issue_worker_token();
    let record = state
        .store
        .create_worker_token(
            org_id,
            Some(profile.id),
            &format!("profile:{}", profile.name),
            &issued.prefix,
            &issued.hash,
            std::slice::from_ref(&profile.harness_kind),
        )
        .await?;
    Ok(IssuedWorkerToken {
        token: issued.token,
        token_id: record.id,
        profile_id: record.profile_id,
        harnesses: record.harnesses,
        name: record.name,
    })
}

#[utoipa::path(post, path = "/api/v1/worker-profiles/{profile_id}/token", tag = "workers", responses((status = 200, body = IssuedWorkerToken)))]
pub async fn rotate_token(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
) -> Result<Json<IssuedWorkerToken>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let profile = state
        .store
        .get_worker_profile(auth.org_id, profile_id)
        .await?;
    Ok(Json(
        issue_profile_token(&state, auth.org_id, &profile, true).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/worker-tokens", tag = "workers", request_body = CreateWorkerTokenRequest, responses((status = 201, body = IssuedWorkerToken)))]
pub async fn create_token(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<CreateWorkerTokenRequest>,
) -> Result<(StatusCode, Json<IssuedWorkerToken>), ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let profile = match body.profile_id {
        Some(profile_id) => Some(
            state
                .store
                .get_worker_profile(auth.org_id, profile_id)
                .await?,
        ),
        None => None,
    };
    let mut harnesses = body.harnesses;
    if let Some(profile) = &profile {
        harnesses = vec![profile.harness_kind.clone()];
    }
    harnesses.sort();
    harnesses.dedup();
    if harnesses.is_empty() {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "harnesses",
            "at least one harness is required",
        )]));
    }
    let issued = issue_worker_token();
    let record = state
        .store
        .create_worker_token(
            auth.org_id,
            body.profile_id,
            body.name.trim(),
            &issued.prefix,
            &issued.hash,
            &harnesses,
        )
        .await?;
    audit::record(
        &state,
        &auth,
        "worker_token.created",
        Some("worker_token"),
        Some(record.id),
        serde_json::json!({ "name": record.name, "harnesses": record.harnesses }),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(IssuedWorkerToken {
            token: issued.token,
            token_id: record.id,
            profile_id: record.profile_id,
            harnesses: record.harnesses,
            name: record.name,
        }),
    ))
}

#[utoipa::path(get, path = "/api/v1/worker-tokens", tag = "workers", responses((status = 200, body = [WorkerTokenView])))]
pub async fn list_tokens(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<WorkerTokenView>>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    Ok(Json(
        state
            .store
            .list_worker_tokens(auth.org_id)
            .await?
            .into_iter()
            .map(|record| WorkerTokenView {
                id: record.id,
                name: record.name,
                profile_id: record.profile_id,
                harnesses: record.harnesses,
                token_prefix: record.token_prefix,
                last_used_at: record.last_used_at,
                revoked_at: record.revoked_at,
                created_at: record.created_at,
            })
            .collect(),
    ))
}

#[utoipa::path(delete, path = "/api/v1/worker-tokens/{token_id}", tag = "workers", responses((status = 204)))]
pub async fn revoke_token(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(token_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    state
        .store
        .revoke_worker_token(auth.org_id, token_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/worker-profiles", tag = "workers", responses((status = 200, body = [WorkerProfile])))]
pub async fn list_profiles(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<WorkerProfile>>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    Ok(Json(state.store.list_worker_profiles(auth.org_id).await?))
}

#[utoipa::path(post, path = "/api/v1/worker-profiles", tag = "workers", request_body = CreateWorkerProfileRequest, responses((status = 201, body = WorkerProfile)))]
pub async fn create_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<CreateWorkerProfileRequest>,
) -> Result<(StatusCode, Json<WorkerProfile>), ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let harness = HarnessKind::from_str_compat(&body.harness_kind)?;
    if !matches!(body.target.as_str(), "local" | "docker" | "kubernetes") {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "target",
            "must be local, docker or kubernetes",
        )]));
    }
    if !matches!(body.execution_driver.as_str(), "process" | "docker") {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "executionDriver",
            "must be process or docker",
        )]));
    }
    let mut issues = Vec::new();
    if body.name.trim().is_empty() {
        issues.push(ValidationIssue::new("name", "name is required"));
    }
    for slug in &body.skill_slugs {
        match state.store.get_skill_by_slug(auth.org_id, slug).await? {
            Some(_) => {}
            None => issues.push(ValidationIssue::new(
                "skillSlugs",
                format!("unknown skill '{slug}'"),
            )),
        }
    }
    for agent_id in &body.agent_ids {
        if state.store.get_agent(auth.org_id, *agent_id).await.is_err() {
            issues.push(ValidationIssue::new(
                "agentIds",
                format!("unknown agent '{agent_id}'"),
            ));
        }
    }
    if !issues.is_empty() {
        return Err(ApiError::validation(issues));
    }
    let image = if body.image.trim().is_empty() {
        crate::images::default_image(harness)
    } else {
        body.image.trim().to_string()
    };
    let profile = state
        .store
        .create_worker_profile(
            auth.org_id,
            body.name.trim(),
            harness.as_str(),
            &body.target,
            &body.execution_driver,
            &image,
            body.image_tag.trim(),
            &body.agent_ids,
            &body.skill_slugs,
            body.provider.as_deref(),
            body.model.as_deref(),
            body.auth_secret_name.as_deref(),
            &body.env,
            &body.env_mappings,
            &body.resources,
        )
        .await?;
    audit::record(
        &state,
        &auth,
        "worker_profile.created",
        Some("worker_profile"),
        Some(profile.id),
        serde_json::json!({ "harness": profile.harness_kind, "target": profile.target }),
    )
    .await;
    Ok((StatusCode::CREATED, Json(profile)))
}

#[utoipa::path(patch, path = "/api/v1/worker-profiles/{profile_id}", tag = "workers", request_body = CreateWorkerProfileRequest, responses((status = 200, body = WorkerProfile)))]
pub async fn update_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<CreateWorkerProfileRequest>,
) -> Result<Json<WorkerProfile>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let harness = HarnessKind::from_str_compat(&body.harness_kind)?;
    if !matches!(body.target.as_str(), "local" | "docker" | "kubernetes") {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "target",
            "must be local, docker or kubernetes",
        )]));
    }
    if !matches!(body.execution_driver.as_str(), "process" | "docker") {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "executionDriver",
            "must be process or docker",
        )]));
    }
    let mut issues = Vec::new();
    if body.name.trim().is_empty() {
        issues.push(ValidationIssue::new("name", "name is required"));
    }
    for slug in &body.skill_slugs {
        if state
            .store
            .get_skill_by_slug(auth.org_id, slug)
            .await?
            .is_none()
        {
            issues.push(ValidationIssue::new(
                "skillSlugs",
                format!("unknown skill '{slug}'"),
            ));
        }
    }
    for agent_id in &body.agent_ids {
        if state.store.get_agent(auth.org_id, *agent_id).await.is_err() {
            issues.push(ValidationIssue::new(
                "agentIds",
                format!("unknown agent '{agent_id}'"),
            ));
        }
    }
    if !issues.is_empty() {
        return Err(ApiError::validation(issues));
    }
    let image = if body.image.trim().is_empty() {
        crate::images::default_image(harness)
    } else {
        body.image.trim().to_string()
    };
    let profile = state
        .store
        .update_worker_profile(
            auth.org_id,
            profile_id,
            body.name.trim(),
            harness.as_str(),
            &body.target,
            &body.execution_driver,
            &image,
            body.image_tag.trim(),
            &body.agent_ids,
            &body.skill_slugs,
            body.provider.as_deref(),
            body.model.as_deref(),
            body.auth_secret_name.as_deref(),
            &body.env,
            &body.env_mappings,
            &body.resources,
        )
        .await?;
    audit::record(
        &state,
        &auth,
        "worker_profile.updated",
        Some("worker_profile"),
        Some(profile.id),
        serde_json::json!({ "harness": profile.harness_kind, "target": profile.target }),
    )
    .await;
    Ok(Json(profile))
}

#[utoipa::path(get, path = "/api/v1/worker-profiles/{profile_id}", tag = "workers", responses((status = 200, body = WorkerProfile)))]
pub async fn get_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
) -> Result<Json<WorkerProfile>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    Ok(Json(
        state
            .store
            .get_worker_profile(auth.org_id, profile_id)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/worker-profiles/{profile_id}/enable", tag = "workers", responses((status = 200, body = WorkerProfile)))]
pub async fn enable_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
) -> Result<Json<WorkerProfile>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    Ok(Json(
        state
            .store
            .set_worker_profile_enabled(auth.org_id, profile_id, true)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/worker-profiles/{profile_id}/disable", tag = "workers", responses((status = 200, body = WorkerProfile)))]
pub async fn disable_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
) -> Result<Json<WorkerProfile>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    Ok(Json(
        state
            .store
            .set_worker_profile_enabled(auth.org_id, profile_id, false)
            .await?,
    ))
}

#[utoipa::path(delete, path = "/api/v1/worker-profiles/{profile_id}", tag = "workers", responses((status = 204), (status = 409)))]
pub async fn delete_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let profile = state
        .store
        .get_worker_profile(auth.org_id, profile_id)
        .await?;
    if profile.enabled {
        return Err(ApiError::conflict(
            "disable the worker profile before deleting it",
        ));
    }
    state
        .store
        .delete_worker_profile(auth.org_id, profile_id)
        .await?;
    audit::record(
        &state,
        &auth,
        "worker_profile.deleted",
        Some("worker_profile"),
        Some(profile_id),
        serde_json::json!({}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(get, path = "/api/v1/worker-profiles/{profile_id}/bootstrap", tag = "workers", responses((status = 200, body = WorkerBootstrap)))]
pub async fn bootstrap(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
) -> Result<Json<WorkerBootstrap>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    let profile = state
        .store
        .get_worker_profile(auth.org_id, profile_id)
        .await?;
    if !profile.enabled {
        return Err(ApiError::conflict(
            "enable the worker profile before generating bootstrap output",
        ));
    }
    let issued = issue_profile_token(&state, auth.org_id, &profile, true).await?;
    let name = format!(
        "orbit-worker-{}-{}",
        profile.harness_kind.replace('_', "-"),
        &profile.id.to_string()[..8]
    );
    let image = format!("{}:{}", profile.image, profile.image_tag);
    let has_auth = profile.auth_secret_name.is_some();

    let mut docker_lines = vec![
        format!("docker run --rm --name {name}"),
        format!("  -e ORBIT_WORKER_PROFILE_ID={}", profile.id),
        format!("  -e ORBIT_WORKER_TOKEN={}", issued.token),
        format!("  -e ORBIT_WORKER_HARNESSES={}", profile.harness_kind),
        format!("  -e ORBIT_WORKER_TARGET={}", profile.target),
        format!("  -e ORBIT_EXECUTION_DRIVER={}", profile.execution_driver),
        "  -e ORBIT_API_URL=http://host.docker.internal:8080".into(),
    ];
    let mut compose_volumes = String::new();
    let mut compose_command = String::new();
    match profile.harness_kind.as_str() {
        "opencode" if has_auth => {
            docker_lines.push("  -v ./auth-export.json:/seed/auth-export.json:ro".into());
            docker_lines.push("  --entrypoint /bin/sh".into());
            docker_lines.push(format!("  {image}"));
            docker_lines.push(
                "  -c 'mkdir -p /home/orbit/.local/share/opencode; opencode auth import --standalone /seed/auth-export.json; exec /usr/local/bin/orbit-worker'"
                    .into(),
            );
            compose_volumes =
                "    volumes:\n      - ./auth-export.json:/seed/auth-export.json:ro\n".into();
            compose_command = "    entrypoint: [\"/bin/sh\", \"-c\", \"mkdir -p /home/orbit/.local/share/opencode; opencode auth import --standalone /seed/auth-export.json; exec /usr/local/bin/orbit-worker\"]\n".into();
        }
        "pi" if has_auth => {
            docker_lines.push("  -v ./pi-agent:/home/orbit/.pi/agent".into());
            docker_lines.push(format!("  {image}"));
            compose_volumes = "    volumes:\n      - ./pi-agent:/home/orbit/.pi/agent\n".into();
        }
        _ => docker_lines.push(format!("  {image}")),
    }
    let docker = docker_lines.join(" \\\n");

    let compose = format!(
        "services:\n  {name}:\n    image: {image}\n    restart: unless-stopped\n    extra_hosts:\n      - \"host.docker.internal:host-gateway\"\n    environment:\n      ORBIT_WORKER_PROFILE_ID: {id}\n      ORBIT_WORKER_TOKEN: {token}\n      ORBIT_WORKER_HARNESSES: {harness}\n      ORBIT_WORKER_TARGET: {target}\n      ORBIT_EXECUTION_DRIVER: {driver}\n      ORBIT_API_URL: http://host.docker.internal:8080\n{volumes}{command}",
        name = name,
        image = image,
        id = profile.id,
        token = issued.token,
        harness = profile.harness_kind,
        target = profile.target,
        driver = profile.execution_driver,
        volumes = compose_volumes,
        command = compose_command
    );

    let kubernetes = format!(
        r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {name}
  namespace: orbit
spec:
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: {name}
  template:
    metadata:
      labels:
        app.kubernetes.io/name: {name}
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 10001
        runAsGroup: 10001
        fsGroup: 10001
      containers:
        - name: worker
          image: {image}
          env:
            - name: ORBIT_WORKER_PROFILE_ID
              value: "{id}"
            - name: ORBIT_WORKER_TOKEN
              value: "{token}"
            - name: ORBIT_WORKER_HARNESSES
              value: "{harness}"
            - name: ORBIT_WORKER_TARGET
              value: "{target}"
            - name: ORBIT_EXECUTION_DRIVER
              value: "{driver}"
            - name: ORBIT_API_URL
              value: "http://orbit-api:8080"
          resources:
            requests: {{ cpu: 100m, memory: 256Mi }}
            limits: {{ cpu: "2", memory: 1Gi }}
          # Mount the harness auth Secret at the adapter's expected path:
          #   OpenCode: /seed/auth-export.json (init import)
          #   Pi:       /home/orbit/.pi/agent
"#,
        name = name,
        image = image,
        id = profile.id,
        token = issued.token,
        harness = profile.harness_kind,
        target = profile.target,
        driver = profile.execution_driver
    );

    let mut notes = vec![
        "The worker loads its agent allowlist and skills from the profile at startup.".into(),
        "The worker receives only ORBIT_API_URL and a scoped, revocable ORBIT_WORKER_TOKEN; no database, NATS or master key.".into(),
        "Generating bootstrap output rotates the profile token; re-run it whenever you add a worker.".into(),
        "Credentials are never included here; mount the harness auth Secret/volume.".into(),
        "Pin the image tag to a release and use the generated manifest as a starting point.".into(),
    ];
    if has_auth {
        notes.push(format!(
            "Profile references auth secret '{}'.",
            profile.auth_secret_name.as_deref().unwrap_or_default()
        ));
    }
    Ok(Json(WorkerBootstrap {
        docker,
        compose,
        kubernetes,
        notes,
    }))
}

trait HarnessKindExt {
    fn from_str_compat(value: &str) -> Result<HarnessKind, ApiError>;
}
impl HarnessKindExt for HarnessKind {
    fn from_str_compat(value: &str) -> Result<HarnessKind, ApiError> {
        value
            .parse()
            .map_err(|_| ApiError::bad_request(format!("unknown harness kind '{value}'")))
    }
}
