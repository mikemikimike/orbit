use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub mode: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub status: String,
    pub oidc_issuer: Option<String>,
    pub oidc_subject: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ApiTokenRecord {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token_prefix: String,
    pub token_hash: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiTokenView {
    pub id: Uuid,
    pub name: String,
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ApiTokenRecord> for ApiTokenView {
    fn from(record: ApiTokenRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            token_prefix: record.token_prefix,
            scopes: record.scopes,
            expires_at: record.expires_at,
            last_used_at: record.last_used_at,
            revoked_at: record.revoked_at,
            created_at: record.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: Uuid,
    pub org_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub current_version: i32,
    pub labels: serde_json::Value,
    pub created_by: Option<Uuid>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AgentWithSpec {
    #[sqlx(flatten)]
    pub agent: Agent,
    pub spec: Option<serde_json::Value>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ConnectionWithConnector {
    #[sqlx(flatten)]
    pub connection: Connection,
    pub connector_slug: String,
    pub connector_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentVersion {
    pub id: Uuid,
    pub org_id: Uuid,
    pub agent_id: Uuid,
    pub version: i32,
    pub spec: serde_json::Value,
    pub definition_yaml: String,
    pub checksum: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRecord {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub implementation_status: String,
    pub capabilities: serde_json::Value,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub kind: String,
    pub config: serde_json::Value,
    #[serde(skip_serializing)]
    pub secret_ref: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorRecord {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub slug: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub auth: serde_json::Value,
    pub capabilities: serde_json::Value,
    pub config_schema: serde_json::Value,
    pub webhooks: bool,
    pub builtin: bool,
    pub docs_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: Uuid,
    pub org_id: Uuid,
    pub connector_id: Uuid,
    pub name: String,
    pub status: String,
    #[serde(skip_serializing)]
    pub credential_ref: Option<String>,
    pub config: serde_json::Value,
    pub scopes: Vec<String>,
    pub created_by: Option<Uuid>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Trigger {
    pub id: Uuid,
    pub org_id: Uuid,
    pub agent_id: Uuid,
    pub kind: String,
    pub enabled: bool,
    pub config: serde_json::Value,
    pub last_fired_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRecord {
    pub id: Uuid,
    pub org_id: Uuid,
    pub agent_id: Uuid,
    pub agent_version_id: Uuid,
    pub agent_name: String,
    pub runtime_kind: String,
    pub trigger_id: Option<Uuid>,
    pub trigger_source: String,
    pub status: String,
    pub session_ref: Option<String>,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<serde_json::Value>,
    pub usage: serde_json::Value,
    pub timeout_seconds: i32,
    pub idempotency_key: Option<String>,
    pub requested_by: Option<Uuid>,
    pub worker_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionEventRecord {
    pub id: i64,
    pub org_id: Uuid,
    pub execution_id: Uuid,
    pub seq: i64,
    pub kind: String,
    pub level: String,
    pub message: String,
    pub data: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogRecord {
    pub id: i64,
    pub org_id: Uuid,
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub request_id: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    pub actor_type: String,
    pub actor_id: Option<Uuid>,
    pub actor_email: Option<String>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub request_id: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentInstance {
    pub id: Uuid,
    pub org_id: Uuid,
    pub agent_id: Uuid,
    pub agent_version_id: Uuid,
    pub runtime_id: Uuid,
    pub name: String,
    pub deployment_target: String,
    pub runtime_ref: Option<String>,
    pub state: String,
    pub health: serde_json::Value,
    pub config: serde_json::Value,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Approval {
    pub id: Uuid,
    pub org_id: Uuid,
    pub execution_id: Uuid,
    pub status: String,
    pub requested_action: serde_json::Value,
    pub decided_by: Option<Uuid>,
    pub decided_at: Option<DateTime<Utc>>,
    pub comment: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalWithExecution {
    #[sqlx(flatten)]
    #[serde(flatten)]
    #[schema(inline)]
    pub approval: Approval,
    pub agent_name: String,
    pub agent_id: Uuid,
    pub execution_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub total_agents: i64,
    pub active_agents: i64,
    pub paused_agents: i64,
    pub draft_agents: i64,
    pub executions_24h: i64,
    pub failed_executions_24h: i64,
    pub running_executions: i64,
    pub total_connections: i64,
    pub enabled_triggers: i64,
    pub recent_executions: Vec<ExecutionRecord>,
    pub pending_approvals: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionWithAgent {
    #[serde(flatten)]
    pub execution: ExecutionRecord,
    pub agent_slug: Option<String>,
}
