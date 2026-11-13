//! Worker-facing control-plane API.
//!
//! Every endpoint is authenticated with an `orbit_wk_` worker token and scoped to the
//! token's organization (and profile, when present). The worker never receives database,
//! NATS or master-key credentials.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use orbit_contracts::{
    subjects, ModelEndpoint, ModelSelection, OrbitEvent, ProviderCompat, ProviderKind, SecretLease,
};
use orbit_domain::{
    AgentSpec, CapabilityEffect, ExecutionEventKind, ExecutionStatus, GrantedCapability, LogLevel,
    ModelManagement,
};
use orbit_store::{
    Approval, Connection, ConnectorRecord, ExecutionRecord, Skill, Store, WorkerProfile,
};

use crate::error::ApiError;
use crate::state::AppState;
use crate::worker_auth::WorkerAuth;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSecret {
    pub secret_ref: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerModel {
    pub selection: ModelSelection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<ModelEndpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<WorkerSecret>,
}

impl WorkerModel {
    pub fn into_resolved(self) -> orbit_contracts::ResolvedModel {
        orbit_contracts::ResolvedModel {
            selection: self.selection,
            endpoint: self.endpoint,
            credential: self
                .credential
                .map(|secret| SecretLease::new(secret.secret_ref, secret.value)),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerExecutionContext {
    pub execution: ExecutionRecord,
    pub spec: AgentSpec,
    pub model: Option<WorkerModel>,
    pub grants: Vec<GrantedCapability>,
    pub skills: Vec<Skill>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerSession {
    pub org_id: Uuid,
    pub profile_id: Option<Uuid>,
    pub profile_name: Option<String>,
    pub harnesses: Vec<String>,
    pub target: Option<String>,
    pub execution_driver: Option<String>,
    pub agent_ids: Vec<Uuid>,
    pub skill_slugs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatRequest {
    pub worker_id: String,
    pub harnesses: Vec<String>,
    pub target: String,
    #[serde(default)]
    pub hostname: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimRequest {
    pub worker_id: String,
    #[serde(default)]
    pub harnesses: Vec<String>,
    pub target: String,
    #[serde(default)]
    pub agent_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimResponse {
    pub execution: Option<WorkerExecutionContext>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningRequest {
    #[serde(default)]
    pub session_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRequest {
    pub kind: String,
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequest {
    pub status: String,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
    #[serde(default)]
    pub usage: Option<serde_json::Value>,
    #[serde(default)]
    pub session_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApprovalRequest {
    pub execution_id: Uuid,
    pub action: serde_json::Value,
    #[serde(default = "default_approval_wait")]
    pub wait_seconds: u64,
}

fn default_approval_wait() -> u64 {
    300
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalQuery {
    pub execution_id: Uuid,
    #[serde(default = "default_approval_wait")]
    pub seconds: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditRequest {
    pub execution_id: Uuid,
    pub action: String,
    #[serde(default)]
    pub connection_id: Option<Uuid>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelResponse {
    pub cancel_requested: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerConnection {
    pub connection: Connection,
    pub connector: ConnectorRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential: Option<WorkerSecret>,
}

async fn active_profile(
    state: &AppState,
    auth: &WorkerAuth,
) -> Result<Option<WorkerProfile>, ApiError> {
    let Some(profile_id) = auth.profile_id else {
        return Ok(None);
    };
    let profile = state
        .store
        .get_worker_profile(auth.org_id, profile_id)
        .await?;
    if !profile.enabled {
        return Err(ApiError::forbidden("worker profile is disabled"));
    }
    Ok(Some(profile))
}

fn intersect_harnesses(token: &WorkerAuth, requested: &[String]) -> Vec<String> {
    let mut requested: Vec<String> = requested
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    if !token.harnesses.is_empty() {
        requested.retain(|harness| token.harnesses.contains(harness));
    }
    requested
}

pub async fn session(
    State(state): State<AppState>,
    auth: WorkerAuth,
) -> Result<Json<WorkerSession>, ApiError> {
    let profile = active_profile(&state, &auth).await?;
    let (harnesses, target, execution_driver, agent_ids, skill_slugs, profile_name) = match &profile
    {
        Some(profile) => (
            vec![profile.harness_kind.clone()],
            Some(profile.target.clone()),
            Some(profile.execution_driver.clone()),
            profile.agent_ids.clone(),
            profile.skill_slugs.clone(),
            Some(profile.name.clone()),
        ),
        None => (
            auth.harnesses.clone(),
            None,
            None,
            Vec::new(),
            Vec::new(),
            None,
        ),
    };
    if harnesses.is_empty() {
        return Err(ApiError::forbidden(
            "worker token has no allowed harnesses; issue a token with a harness list",
        ));
    }
    Ok(Json(WorkerSession {
        org_id: auth.org_id,
        profile_id: auth.profile_id,
        profile_name,
        harnesses,
        target,
        execution_driver,
        agent_ids,
        skill_slugs,
    }))
}

pub async fn offline(
    State(state): State<AppState>,
    auth: WorkerAuth,
    Path(worker_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let _ = active_profile(&state, &auth).await?;
    state.store.delete_worker_heartbeat(&worker_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn heartbeat(
    State(state): State<AppState>,
    auth: WorkerAuth,
    crate::extract::ApiJson(body): crate::extract::ApiJson<HeartbeatRequest>,
) -> Result<StatusCode, ApiError> {
    let _ = active_profile(&state, &auth).await?;
    let harnesses = intersect_harnesses(&auth, &body.harnesses);
    state
        .store
        .upsert_worker_heartbeat(
            &body.worker_id,
            &harnesses,
            &body.target,
            body.hostname.as_deref(),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn claim(
    State(state): State<AppState>,
    auth: WorkerAuth,
    crate::extract::ApiJson(body): crate::extract::ApiJson<ClaimRequest>,
) -> Result<Json<ClaimResponse>, ApiError> {
    let profile = active_profile(&state, &auth).await?;
    let harnesses = match &profile {
        Some(profile) => vec![profile.harness_kind.clone()],
        None => intersect_harnesses(&auth, &body.harnesses),
    };
    if harnesses.is_empty() {
        return Ok(Json(ClaimResponse { execution: None }));
    }
    let agent_ids = profile
        .as_ref()
        .map(|profile| profile.agent_ids.clone())
        .unwrap_or(body.agent_ids);
    let execution = match state
        .store
        .claim_next_queued(&body.worker_id, &harnesses, &body.target, &agent_ids)
        .await?
    {
        Some(execution) => execution,
        None => return Ok(Json(ClaimResponse { execution: None })),
    };
    match build_context(&state, auth.org_id, profile.as_ref(), &execution).await {
        Ok(context) => Ok(Json(ClaimResponse {
            execution: Some(context),
        })),
        Err(error) => {
            let message = error.message.clone();
            fail_claimed(&state, &execution, &message).await;
            Err(error)
        }
    }
}

pub async fn context(
    State(state): State<AppState>,
    auth: WorkerAuth,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<WorkerExecutionContext>, ApiError> {
    let profile = active_profile(&state, &auth).await?;
    let execution = state.store.get_execution_by_id(execution_id).await?;
    if execution.org_id != auth.org_id {
        return Err(ApiError::not_found("execution"));
    }
    if let Some(profile) = &profile {
        if !profile.agent_ids.is_empty() && !profile.agent_ids.contains(&execution.agent_id) {
            return Err(ApiError::forbidden(
                "execution agent is outside this worker profile",
            ));
        }
    }
    Ok(Json(
        build_context(&state, auth.org_id, profile.as_ref(), &execution).await?,
    ))
}

async fn build_context(
    state: &AppState,
    org_id: Uuid,
    profile: Option<&WorkerProfile>,
    execution: &ExecutionRecord,
) -> Result<WorkerExecutionContext, ApiError> {
    let version = state
        .store
        .get_agent_version_by_id(org_id, execution.agent_version_id)
        .await?;
    let spec: AgentSpec = serde_json::from_value(version.spec)
        .map_err(|error| ApiError::internal(format!("stored agent spec is invalid: {error}")))?;
    if let Some(record) = state
        .store
        .get_harness_by_kind(org_id, spec.harness.kind.as_str())
        .await?
    {
        if !record.enabled {
            return Err(ApiError::conflict(format!(
                "harness '{}' is disabled",
                spec.harness.kind
            )));
        }
    }
    let model = resolve_model(&state.store, org_id, &spec).await?;
    let grants = resolve_grants(&state.store, org_id, &spec).await;
    let mut slugs = spec.skills.clone();
    slugs.extend(execution.skills.iter().cloned());
    if let Some(profile) = profile {
        slugs.extend(profile.skill_slugs.iter().cloned());
    }
    slugs.sort();
    slugs.dedup();
    let skills = state.store.skills_by_slugs(org_id, &slugs).await?;
    Ok(WorkerExecutionContext {
        execution: execution.clone(),
        spec,
        model,
        grants,
        skills,
    })
}

async fn fail_claimed(state: &AppState, execution: &ExecutionRecord, message: &str) {
    let event = state
        .store
        .append_execution_event(
            execution.org_id,
            execution.id,
            ExecutionEventKind::Error.as_str(),
            LogLevel::Error,
            message,
            &serde_json::json!({ "error": message }),
        )
        .await;
    if let Ok(event) = &event {
        publish_event(state, execution, event).await;
    }
    if let Ok(Some(record)) = state
        .store
        .finish_execution(
            execution.id,
            ExecutionStatus::Failed,
            None,
            Some(&serde_json::json!({ "code": "invalid_config", "message": message })),
            None,
            None,
        )
        .await
    {
        publish_status(state, &record).await;
    }
}

pub async fn running(
    State(state): State<AppState>,
    auth: WorkerAuth,
    Path(execution_id): Path<Uuid>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<RunningRequest>,
) -> Result<StatusCode, ApiError> {
    ensure_execution(&state, &auth, execution_id).await?;
    state
        .store
        .mark_execution_running(execution_id, body.session_ref.as_deref())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn event(
    State(state): State<AppState>,
    auth: WorkerAuth,
    Path(execution_id): Path<Uuid>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<EventRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let execution = ensure_execution(&state, &auth, execution_id).await?;
    let level = match body.level.as_str() {
        "debug" => LogLevel::Debug,
        "info" => LogLevel::Info,
        "warn" => LogLevel::Warn,
        "error" => LogLevel::Error,
        other => {
            return Err(ApiError::bad_request(format!(
                "unknown log level '{other}'"
            )))
        }
    };
    let event = state
        .store
        .append_execution_event(
            execution.org_id,
            execution.id,
            &body.kind,
            level,
            &body.message,
            &body.data,
        )
        .await?;
    publish_event(&state, &execution, &event).await;
    Ok(Json(serde_json::to_value(event).unwrap_or_default()))
}

pub async fn complete(
    State(state): State<AppState>,
    auth: WorkerAuth,
    Path(execution_id): Path<Uuid>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<CompleteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    ensure_execution(&state, &auth, execution_id).await?;
    let status = ExecutionStatus::parse(&body.status)
        .ok_or_else(|| ApiError::bad_request(format!("unknown status '{}'", body.status)))?;
    if !status.is_terminal() {
        return Err(ApiError::bad_request(
            "worker completion requires a terminal status",
        ));
    }
    let updated = state
        .store
        .finish_execution(
            execution_id,
            status,
            body.output.as_ref(),
            body.error.as_ref(),
            body.usage.as_ref(),
            body.session_ref.as_deref(),
        )
        .await?;
    if let Some(record) = &updated {
        publish_status(&state, record).await;
    }
    Ok(Json(serde_json::json!({
        "updated": updated.is_some(),
        "execution": updated,
    })))
}

pub async fn cancel_requested(
    State(state): State<AppState>,
    auth: WorkerAuth,
    Path(execution_id): Path<Uuid>,
) -> Result<Json<CancelResponse>, ApiError> {
    ensure_execution(&state, &auth, execution_id).await?;
    Ok(Json(CancelResponse {
        cancel_requested: state.store.is_cancel_requested(execution_id).await?,
    }))
}

pub async fn create_approval(
    State(state): State<AppState>,
    auth: WorkerAuth,
    crate::extract::ApiJson(body): crate::extract::ApiJson<CreateApprovalRequest>,
) -> Result<Json<Approval>, ApiError> {
    let execution = ensure_execution(&state, &auth, body.execution_id).await?;
    let approval = state
        .store
        .create_approval(
            auth.org_id,
            execution.id,
            body.action,
            Some(Utc::now() + chrono::Duration::seconds(body.wait_seconds as i64)),
        )
        .await?;
    Ok(Json(approval))
}

pub async fn await_approval(
    State(state): State<AppState>,
    auth: WorkerAuth,
    Path(approval_id): Path<Uuid>,
    Query(query): Query<ApprovalQuery>,
) -> Result<Json<Approval>, ApiError> {
    ensure_execution(&state, &auth, query.execution_id).await?;
    Ok(Json(
        state
            .store
            .await_approval(
                auth.org_id,
                approval_id,
                std::time::Duration::from_secs(query.seconds.clamp(1, 900)),
            )
            .await?,
    ))
}

pub async fn audit(
    State(state): State<AppState>,
    auth: WorkerAuth,
    crate::extract::ApiJson(body): crate::extract::ApiJson<AuditRequest>,
) -> Result<StatusCode, ApiError> {
    ensure_execution(&state, &auth, body.execution_id).await?;
    state
        .store
        .append_audit_log(
            auth.org_id,
            "worker",
            None,
            None,
            &body.action,
            Some("connection"),
            body.connection_id,
            body.metadata,
            None,
            None,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionQuery {
    pub execution_id: Uuid,
}

pub async fn connection(
    State(state): State<AppState>,
    auth: WorkerAuth,
    Path(connection_id): Path<Uuid>,
    Query(query): Query<ConnectionQuery>,
) -> Result<Json<WorkerConnection>, ApiError> {
    let execution = ensure_execution(&state, &auth, query.execution_id).await?;
    let version = state
        .store
        .get_agent_version_by_id(auth.org_id, execution.agent_version_id)
        .await?;
    let spec: AgentSpec = serde_json::from_value(version.spec)
        .map_err(|error| ApiError::internal(format!("stored agent spec is invalid: {error}")))?;
    if !spec
        .connectors
        .iter()
        .any(|binding| binding.connection_id == Some(connection_id))
    {
        return Err(ApiError::forbidden(
            "connection is not bound to this execution",
        ));
    }
    let connection = state
        .store
        .get_connection_with_connector(auth.org_id, connection_id)
        .await?;
    let connector = state
        .store
        .get_connector_by_slug(auth.org_id, &connection.connector_slug)
        .await?
        .ok_or_else(|| ApiError::not_found("connector"))?;
    let credential = match &connection.connection.credential_ref {
        Some(reference) => {
            state
                .store
                .get_secret_value(reference)
                .await?
                .map(|value| WorkerSecret {
                    secret_ref: reference.clone(),
                    value: value.expose().to_string(),
                })
        }
        None => None,
    };
    Ok(Json(WorkerConnection {
        connection: connection.connection,
        connector,
        credential,
    }))
}

async fn ensure_execution(
    state: &AppState,
    auth: &WorkerAuth,
    execution_id: Uuid,
) -> Result<ExecutionRecord, ApiError> {
    let execution = state.store.get_execution_by_id(execution_id).await?;
    if execution.org_id != auth.org_id {
        return Err(ApiError::not_found("execution"));
    }
    Ok(execution)
}

async fn publish_event(
    state: &AppState,
    execution: &ExecutionRecord,
    event: &orbit_store::ExecutionEventRecord,
) {
    let envelope = OrbitEvent::new(
        subjects::EXECUTION_EVENT,
        "orbit-api-worker",
        serde_json::to_value(event).unwrap_or_default(),
    )
    .with_org(execution.org_id)
    .with_idempotency(format!("event:{}", event.id));
    let _ = state.bus.publish(envelope).await;
}

async fn publish_status(state: &AppState, execution: &ExecutionRecord) {
    let event = OrbitEvent::new(
        subjects::EXECUTION_UPDATED,
        "orbit-api-worker",
        serde_json::json!({
            "executionId": execution.id,
            "agentId": execution.agent_id,
            "status": execution.status,
            "finishedAt": execution.finished_at,
        }),
    )
    .with_org(execution.org_id)
    .with_idempotency(format!("status:{}:{}", execution.id, execution.status));
    let _ = state.bus.publish(event).await;
}

async fn resolve_model(
    store: &Store,
    org_id: Uuid,
    spec: &AgentSpec,
) -> Result<Option<WorkerModel>, ApiError> {
    let provider_name = spec.model.provider.clone();
    let model = spec.model.model.clone();

    if spec.model.management == ModelManagement::Harness {
        let mut resolved = WorkerModel {
            selection: ModelSelection::Harness {
                provider: provider_name.clone(),
                model,
            },
            endpoint: None,
            credential: None,
        };
        if let Some(name) = &provider_name {
            if let Some(provider) = store.get_provider_by_name(org_id, name).await? {
                if let Some(reference) = &provider.secret_ref {
                    let value = store.get_secret_value(reference).await?.ok_or_else(|| {
                        ApiError::conflict(format!(
                            "credential for provider '{name}' is missing from the secret store"
                        ))
                    })?;
                    resolved.credential = Some(WorkerSecret {
                        secret_ref: reference.clone(),
                        value: value.expose().to_string(),
                    });
                }
            }
        }
        return Ok(Some(resolved));
    }

    let name = provider_name.ok_or_else(|| {
        ApiError::conflict("model management is 'platform' but no provider is configured")
    })?;
    let provider = store
        .get_provider_by_name(org_id, &name)
        .await?
        .ok_or_else(|| ApiError::conflict(format!("provider '{name}' does not exist")))?;
    let kind = ProviderKind::parse(&provider.kind)
        .ok_or_else(|| ApiError::internal("provider has an unknown kind"))?;
    if kind != ProviderKind::OpenAiCompatible {
        return Err(ApiError::conflict(format!(
            "provider '{name}' is harness-managed and cannot be used with platform model management"
        )));
    }
    if !provider.enabled {
        return Err(ApiError::conflict(format!("provider '{name}' is disabled")));
    }
    let base_url = provider
        .config
        .get("baseUrl")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ApiError::conflict(format!("provider '{name}' is missing baseUrl")))?
        .to_string();
    let model_id = model
        .or_else(|| {
            provider
                .config
                .get("model")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .ok_or_else(|| ApiError::conflict(format!("provider '{name}' is missing a model id")))?;
    let mut headers = BTreeMap::new();
    if let Some(map) = provider
        .config
        .get("headers")
        .and_then(|value| value.as_object())
    {
        for (key, value) in map {
            if let Some(value) = value.as_str() {
                headers.insert(key.clone(), value.to_string());
            }
        }
    }
    let compat: ProviderCompat = provider
        .config
        .get("compat")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();
    let request_params = provider
        .config
        .get("requestParams")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let context_window = provider
        .config
        .get("contextWindow")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);
    let max_output_tokens = provider
        .config
        .get("maxOutputTokens")
        .and_then(|value| value.as_u64())
        .map(|value| value as u32);
    let credential = match &provider.secret_ref {
        Some(reference) => Some(WorkerSecret {
            secret_ref: reference.clone(),
            value: store
                .get_secret_value(reference)
                .await?
                .ok_or_else(|| {
                    ApiError::conflict(format!(
                        "credential for provider '{name}' is missing from the secret store"
                    ))
                })?
                .expose()
                .to_string(),
        }),
        None => None,
    };
    Ok(Some(WorkerModel {
        selection: ModelSelection::Platform {
            provider_id: provider.id,
            provider_name: provider.name,
            model: model_id.clone(),
        },
        endpoint: Some(ModelEndpoint {
            base_url,
            model: model_id,
            headers,
            request_params,
            compat,
            context_window,
            max_output_tokens,
        }),
        credential,
    }))
}

async fn resolve_grants(store: &Store, org_id: Uuid, spec: &AgentSpec) -> Vec<GrantedCapability> {
    let mut grants = Vec::new();
    for binding in &spec.connectors {
        let connector = match store
            .get_connector_by_slug(org_id, &binding.connector)
            .await
        {
            Ok(Some(connector)) => connector,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(%error, connector = %binding.connector, "grant resolution failed");
                continue;
            }
        };
        for capability in &binding.capabilities {
            let effect = connector
                .capabilities
                .as_array()
                .and_then(|items| {
                    items.iter().find(|item| {
                        item.get("id").and_then(|id| id.as_str()) == Some(capability.as_str())
                    })
                })
                .and_then(|item| item.get("effect").and_then(|effect| effect.as_str()))
                .and_then(CapabilityEffect::parse)
                .unwrap_or(CapabilityEffect::Read);
            grants.push(GrantedCapability {
                connection_id: binding.connection_id.unwrap_or(Uuid::nil()),
                connector: binding.connector.clone(),
                capability: capability.clone(),
                effect,
            });
        }
    }
    grants
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    pub id: uuid::Uuid,
    pub kind: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMemoryRequest {
    #[serde(default)]
    pub agent_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub retention_days: Option<u32>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreMemoryRequest {
    pub agent_id: uuid::Uuid,
    pub kind: String,
    pub content: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

pub async fn list_memory(
    State(state): State<AppState>,
    auth: WorkerAuth,
    crate::extract::ApiJson(body): crate::extract::ApiJson<ListMemoryRequest>,
) -> Result<Json<Vec<MemoryEntry>>, ApiError> {
    let org_id = auth.org_id;
    let agent_id = match body.agent_id {
        Some(id) => id,
        None => return Ok(Json(vec![])),
    };
    if agent_id.is_nil() {
        return Ok(Json(vec![]));
    }
    let entries = state.store.list_memory(org_id, agent_id, None, 20).await?;
    Ok(Json(
        entries
            .into_iter()
            .map(|e| MemoryEntry {
                id: e.id,
                kind: e.kind,
                content: e.content,
                created_at: e.created_at,
            })
            .collect(),
    ))
}

pub async fn store_memory(
    State(state): State<AppState>,
    auth: WorkerAuth,
    crate::extract::ApiJson(body): crate::extract::ApiJson<StoreMemoryRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let org_id = auth.org_id;
    let expires_at = None;
    state
        .store
        .store_memory(
            org_id,
            body.agent_id,
            &body.kind,
            &body.content,
            &body.metadata,
            expires_at,
        )
        .await?;
    Ok(Json(serde_json::json!({ "stored": true })))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvMapping {
    pub env_var: String,
    #[serde(default)]
    pub connection_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub secret_key: Option<String>,
    #[serde(default)]
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedEnv {
    pub env_var: String,
    pub value: String,
}

pub async fn resolve_env(
    State(state): State<AppState>,
    auth: WorkerAuth,
) -> Result<Json<Vec<ResolvedEnv>>, ApiError> {
    // Get the worker's profile to find env_mappings
    let profile = match auth.profile_id {
        Some(id) => state.store.get_worker_profile(auth.org_id, id).await.ok(),
        None => None,
    };
    let mappings: Vec<EnvMapping> = profile
        .map(|p| serde_json::from_value(p.env_mappings).unwrap_or_default())
        .unwrap_or_default();

    let mut resolved = Vec::new();
    for mapping in mappings {
        let value = if let Some(conn_id) = mapping.connection_id {
            // Fetch connection and extract the secret value
            match state.store.get_connection(auth.org_id, conn_id).await {
                Ok(conn) => {
                    if let Some(ref secret_ref) = conn.credential_ref {
                        state
                            .store
                            .get_secret_value(secret_ref)
                            .await
                            .ok()
                            .flatten()
                            .map(|s| s.expose().to_string())
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        } else if let Some(ref secret_ref) = mapping.secret_ref {
            state
                .store
                .get_secret_value(secret_ref)
                .await
                .ok()
                .flatten()
                .map(|s| s.expose().to_string())
        } else {
            None
        };
        if let Some(value) = value {
            resolved.push(ResolvedEnv {
                env_var: mapping.env_var,
                value,
            });
        }
    }
    Ok(Json(resolved))
}
