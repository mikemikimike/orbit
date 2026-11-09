use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use orbit_domain::{RuntimeKind, ValidationReport};

/// Capabilities a runtime advertises. The UI and dispatch logic must branch on these,
/// never on a hardcoded runtime name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub supports_sessions: bool,
    pub supports_resume: bool,
    pub supports_streaming: bool,
    pub supports_cancel: bool,
    pub supports_instances: bool,
    pub supports_tool_events: bool,
    pub supports_usage_reporting: bool,
    /// The runtime owns its provider inventory and credentials (OpenCode Go, Claude Code).
    pub runtime_managed_providers: bool,
    /// Runtimes that can consume a provider configured by ORBIT.
    pub supports_platform_providers: bool,
    pub execution_modes: Vec<orbit_domain::ExecutionMode>,
    pub max_timeout_seconds: u64,
    pub max_memory_mb: u32,
}

impl Default for RuntimeCapabilities {
    fn default() -> Self {
        Self {
            supports_sessions: true,
            supports_resume: false,
            supports_streaming: true,
            supports_cancel: true,
            supports_instances: false,
            supports_tool_events: false,
            supports_usage_reporting: false,
            runtime_managed_providers: false,
            supports_platform_providers: true,
            execution_modes: vec![orbit_domain::ExecutionMode::OnDemand],
            max_timeout_seconds: 3600,
            max_memory_mb: 8192,
        }
    }
}

/// Static description of a runtime family, exposed through `GET /api/v1/runtimes`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDescriptor {
    pub kind: RuntimeKind,
    pub name: String,
    pub description: String,
    pub capabilities: RuntimeCapabilities,
    pub implementation_status: orbit_domain::ImplementationStatus,
    pub config_schema: serde_json::Value,
    #[serde(default)]
    pub docs_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    pub kind: RuntimeKind,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceState {
    Pending,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceHealth {
    pub healthy: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl InstanceHealth {
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            message: None,
        }
    }
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeHealth {
    pub healthy: bool,
    pub message: Option<String>,
    #[serde(default)]
    pub details: BTreeMap<String, String>,
}

impl RuntimeHealth {
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            message: None,
            details: BTreeMap::new(),
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: Some(message.into()),
            details: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSpec {
    /// Per-execution scratch directory. Must be writable and isolated.
    pub root: String,
    #[serde(default)]
    pub read_only: bool,
}

impl Default for WorkspaceSpec {
    fn default() -> Self {
        Self {
            root: "/workspace".to_string(),
            read_only: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceRequest {
    pub agent_id: Uuid,
    pub agent_version_id: Uuid,
    pub agent_name: String,
    pub runtime_config: serde_json::Value,
    pub resources: orbit_domain::ResourceLimits,
    #[serde(default)]
    pub workspace: WorkspaceSpec,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstance {
    pub id: String,
    pub runtime_kind: RuntimeKind,
    pub state: InstanceState,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstanceRef {
    pub runtime_kind: RuntimeKind,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstanceStatus {
    pub state: InstanceState,
    pub health: InstanceHealth,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub checked_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstructionsBlock {
    /// Trusted system instructions. Must never be concatenated with untrusted content
    /// by the adapter without clear delimiting/provenance metadata.
    pub system: String,
    #[serde(default)]
    pub constraints: Vec<String>,
}

/// Provenance of execution input. External content is never trusted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputProvenance {
    pub source: String,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub subject: Option<String>,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

impl InputProvenance {
    pub fn manual(user: impl Into<String>) -> Self {
        Self {
            source: format!("user:{}", user.into()),
            trusted: true,
            subject: None,
            received_at: chrono::Utc::now(),
        }
    }

    pub fn external(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            trusted: false,
            subject: None,
            received_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionInput {
    /// The task or message.
    pub task: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub provenance: InputProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRequest {
    #[serde(default)]
    pub session_ref: Option<String>,
    #[serde(default)]
    pub mode: orbit_domain::ExecutionMode,
}

impl Default for SessionRequest {
    fn default() -> Self {
        Self {
            session_ref: None,
            mode: orbit_domain::ExecutionMode::OnDemand,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRequest {
    pub execution_id: Uuid,
    pub agent_id: Uuid,
    pub agent_version_id: Uuid,
    pub agent_name: String,
    pub instructions: InstructionsBlock,
    pub input: ExecutionInput,
    /// Resolved model/provider selection. Runtime-managed runtimes may receive `None`
    /// and use their own inventory.
    #[serde(default)]
    pub model: Option<crate::provider::ResolvedModel>,
    #[serde(default)]
    pub grants: Vec<orbit_domain::GrantedCapability>,
    pub autonomy: orbit_domain::AutonomyLevel,
    pub resources: orbit_domain::ResourceLimits,
    #[serde(default)]
    pub runtime_config: serde_json::Value,
    #[serde(default)]
    pub session: SessionRequest,
    #[serde(default)]
    pub workspace: WorkspaceSpec,
    /// Incremental runtime events. Required for streaming; adapters may ignore it when
    /// `capabilities().supports_streaming` is false.
    #[serde(skip)]
    pub events: Option<tokio::sync::mpsc::Sender<RuntimeEvent>>,
    #[serde(skip, default = "default_cancellation")]
    pub cancel: tokio_util::sync::CancellationToken,
}

fn default_cancellation() -> tokio_util::sync::CancellationToken {
    tokio_util::sync::CancellationToken::new()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeRequest {
    pub execution_id: Uuid,
    pub session_ref: String,
    pub input: ExecutionInput,
    #[serde(skip)]
    pub events: Option<tokio::sync::mpsc::Sender<RuntimeEvent>>,
    #[serde(skip, default = "default_cancellation")]
    pub cancel: tokio_util::sync::CancellationToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventKind {
    Started,
    Status,
    Log,
    ToolCall,
    OutputDelta,
    Usage,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEvent {
    pub kind: RuntimeEventKind,
    pub level: orbit_domain::LogLevel,
    pub message: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

impl RuntimeEvent {
    pub fn new(
        kind: RuntimeEventKind,
        level: orbit_domain::LogLevel,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            level,
            message: message.into(),
            data: serde_json::Value::Null,
        }
    }

    pub fn log(level: orbit_domain::LogLevel, message: impl Into<String>) -> Self {
        Self::new(RuntimeEventKind::Log, level, message)
    }

    pub fn started(session_ref: &str) -> Self {
        let mut event = Self::new(
            RuntimeEventKind::Started,
            orbit_domain::LogLevel::Info,
            format!("runtime session {session_ref} started"),
        );
        event.data = serde_json::json!({ "sessionRef": session_ref });
        event
    }

    pub fn output(text: impl Into<String>) -> Self {
        Self::new(
            RuntimeEventKind::OutputDelta,
            orbit_domain::LogLevel::Info,
            text,
        )
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionOutcome {
    pub status: orbit_domain::ExecutionStatus,
    #[serde(default)]
    pub output: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<RuntimeError>,
    #[serde(default)]
    pub usage: orbit_domain::Usage,
    #[serde(default)]
    pub session_ref: Option<String>,
}

impl ExecutionOutcome {
    pub fn succeeded(output: serde_json::Value, session_ref: Option<String>) -> Self {
        Self {
            status: orbit_domain::ExecutionStatus::Succeeded,
            output: Some(output),
            error: None,
            usage: default_usage(),
            session_ref,
        }
    }

    pub fn failed(error: RuntimeError) -> Self {
        Self {
            status: orbit_domain::ExecutionStatus::Failed,
            output: None,
            error: Some(error),
            usage: default_usage(),
            session_ref: None,
        }
    }
}

fn default_usage() -> orbit_domain::Usage {
    orbit_domain::Usage::default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeErrorKind {
    InvalidConfig,
    Unauthorized,
    Unavailable,
    Unsupported,
    Timeout,
    Cancelled,
    ExecutionFailed,
    Protocol,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub message: String,
    #[serde(default)]
    pub retryable: bool,
    #[serde(default)]
    pub details: serde_json::Value,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable: false,
            details: serde_json::Value::Null,
        }
    }

    pub fn unsupported(what: impl fmt::Display) -> Self {
        Self::new(
            RuntimeErrorKind::Unsupported,
            format!("runtime does not support {what}"),
        )
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        let mut err = Self::new(RuntimeErrorKind::Unavailable, message);
        err.retryable = true;
        err
    }

    pub fn cancelled() -> Self {
        Self::new(RuntimeErrorKind::Cancelled, "execution cancelled")
    }

    pub fn timeout(seconds: u64) -> Self {
        Self::new(
            RuntimeErrorKind::Timeout,
            format!("execution timed out after {seconds}s"),
        )
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// Adapter contract. See `docs/architecture/runtime-contract.md`.
#[async_trait::async_trait]
pub trait RuntimeAdapter: Send + Sync {
    fn kind(&self) -> RuntimeKind;
    fn descriptor(&self) -> RuntimeDescriptor;
    fn capabilities(&self) -> RuntimeCapabilities {
        self.descriptor().capabilities
    }

    /// Validate runtime-specific configuration. Returns issues instead of failing, so
    /// the UI can present all problems at once.
    async fn validate_config(&self, config: &RuntimeConfig) -> ValidationReport;

    async fn health(&self, config: &RuntimeConfig) -> Result<RuntimeHealth, RuntimeError>;

    async fn create_instance(
        &self,
        request: CreateInstanceRequest,
    ) -> Result<RuntimeInstance, RuntimeError>;

    async fn start_instance(&self, instance: &RuntimeInstanceRef) -> Result<(), RuntimeError>;
    async fn stop_instance(&self, instance: &RuntimeInstanceRef) -> Result<(), RuntimeError>;
    async fn delete_instance(&self, instance: &RuntimeInstanceRef) -> Result<(), RuntimeError>;
    async fn instance_status(
        &self,
        instance: &RuntimeInstanceRef,
    ) -> Result<RuntimeInstanceStatus, RuntimeError>;

    /// Run one execution to completion. Must honour cancellation and timeouts and clean
    /// up all child processes/containers before returning.
    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionOutcome, RuntimeError>;

    async fn resume(&self, request: ResumeRequest) -> Result<ExecutionOutcome, RuntimeError> {
        let _ = request;
        Err(RuntimeError::unsupported("session resumption"))
    }
}
