use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use orbit_domain::{HarnessKind, ValidationIssue};
use orbit_security::{AuthContext, Permission};
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
        default_image(harness)
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

#[utoipa::path(delete, path = "/api/v1/worker-profiles/{profile_id}", tag = "workers", responses((status = 204)))]
pub async fn delete_profile(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(profile_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require(Permission::SettingsWrite)?;
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
    auth.require(Permission::HarnessesRead)?;
    let profile = state
        .store
        .get_worker_profile(auth.org_id, profile_id)
        .await?;
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
        format!("  -e ORBIT_WORKER_HARNESSS={}", profile.harness_kind),
        format!("  -e ORBIT_WORKER_TARGET={}", profile.target),
        format!("  -e ORBIT_EXECUTION_DRIVER={}", profile.execution_driver),
        "  -e ORBIT_DATABASE_URL='postgres://orbit:CHANGE_ME@host.docker.internal:5432/orbit'"
            .into(),
        "  -e ORBIT_NATS_URL='nats://host.docker.internal:4222'".into(),
        "  -e ORBIT_MASTER_KEY='CHANGE_ME'".into(),
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
        "services:\n  {name}:\n    image: {image}\n    restart: unless-stopped\n    environment:\n      ORBIT_WORKER_PROFILE_ID: {id}\n      ORBIT_WORKER_HARNESSS: {harness}\n      ORBIT_WORKER_TARGET: {target}\n      ORBIT_EXECUTION_DRIVER: {driver}\n      ORBIT_DATABASE_URL: ${{ORBIT_DATABASE_URL}}\n      ORBIT_NATS_URL: ${{ORBIT_NATS_URL}}\n      ORBIT_MASTER_KEY: ${{ORBIT_MASTER_KEY}}\n{volumes}{command}",
        name = name,
        image = image,
        id = profile.id,
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
            - name: ORBIT_WORKER_HARNESSS
              value: "{harness}"
            - name: ORBIT_WORKER_TARGET
              value: "{target}"
            - name: ORBIT_EXECUTION_DRIVER
              value: "{driver}"
            - name: ORBIT_DATABASE_URL
              valueFrom:
                secretKeyRef: {{ name: orbit-secrets, key: database-url }}
            - name: ORBIT_MASTER_KEY
              valueFrom:
                secretKeyRef: {{ name: orbit-secrets, key: master-key }}
            - name: ORBIT_NATS_URL
              value: "nats://orbit-nats:4222"
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
        harness = profile.harness_kind,
        target = profile.target,
        driver = profile.execution_driver
    );

    let mut notes = vec![
        "The worker loads its agent allowlist and skills from the profile at startup.".into(),
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

fn default_image(harness: HarnessKind) -> String {
    match harness {
        HarnessKind::Pi => "192.168.1.201:5000/orbit-worker-pi".into(),
        HarnessKind::OpenCode => "192.168.1.201:5000/orbit-worker-opencode".into(),
        _ => "192.168.1.201:5000/orbit-worker".into(),
    }
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
