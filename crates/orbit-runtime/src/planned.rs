use async_trait::async_trait;

use orbit_contracts::{
    CreateInstanceRequest, ExecutionOutcome, ExecutionRequest, RuntimeAdapter, RuntimeCapabilities,
    RuntimeConfig, RuntimeDescriptor, RuntimeError, RuntimeErrorKind, RuntimeHealth,
    RuntimeInstance, RuntimeInstanceRef, RuntimeInstanceStatus,
};
use orbit_domain::{ExecutionMode, ImplementationStatus, RuntimeKind, ValidationReport};

/// Adapter for runtimes that are registered (so agents can reference them and the UI can
/// show capabilities) but are not implemented in this build. Execution fails with a
/// precise, non-retryable error instead of pretending to work.
#[derive(Debug, Clone)]
pub struct DescriptorOnlyAdapter {
    descriptor: RuntimeDescriptor,
}

impl DescriptorOnlyAdapter {
    pub fn new(descriptor: RuntimeDescriptor) -> Self {
        Self { descriptor }
    }
}

pub fn claude_code() -> DescriptorOnlyAdapter {
    DescriptorOnlyAdapter::new(RuntimeDescriptor {
        kind: RuntimeKind::ClaudeCode,
        name: "Claude Code".into(),
        description: "Claude Code CLI runtime (headless/programmatic). Adapter lands in Phase 2; \
             authentication and licensing are runtime-managed."
            .into(),
        capabilities: RuntimeCapabilities {
            supports_sessions: true,
            supports_resume: true,
            supports_streaming: true,
            supports_cancel: true,
            supports_instances: false,
            supports_tool_events: true,
            supports_usage_reporting: true,
            runtime_managed_providers: true,
            supports_platform_providers: false,
            execution_modes: vec![ExecutionMode::OnDemand],
            max_timeout_seconds: 86_400,
            max_memory_mb: 16_384,
        },
        implementation_status: ImplementationStatus::Planned,
        config_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "binary": { "type": "string" },
                "permissionMode": { "type": "string", "enum": ["default", "acceptEdits", "plan", "bypassPermissions"] },
                "allowedTools": { "type": "array", "items": { "type": "string" } },
                "extraArgs": { "type": "array", "items": { "type": "string" } }
            }
        }),
        docs_url: Some("https://docs.anthropic.com/en/docs/claude-code".into()),
    })
}

pub fn pi() -> DescriptorOnlyAdapter {
    DescriptorOnlyAdapter::new(RuntimeDescriptor {
        kind: RuntimeKind::Pi,
        name: "Pi".into(),
        description: "Pi runtime adapter (Phase 3).".into(),
        capabilities: RuntimeCapabilities {
            supports_sessions: true,
            supports_resume: true,
            supports_streaming: true,
            supports_cancel: true,
            supports_instances: true,
            supports_tool_events: true,
            supports_usage_reporting: true,
            runtime_managed_providers: true,
            supports_platform_providers: true,
            execution_modes: vec![ExecutionMode::OnDemand, ExecutionMode::LongRunning],
            max_timeout_seconds: 86_400,
            max_memory_mb: 16_384,
        },
        implementation_status: ImplementationStatus::Planned,
        config_schema: serde_json::json!({ "type": "object" }),
        docs_url: None,
    })
}

pub fn generic() -> DescriptorOnlyAdapter {
    DescriptorOnlyAdapter::new(RuntimeDescriptor {
        kind: RuntimeKind::Generic,
        name: "Generic runtime".into(),
        description: "Bring-your-own runtime speaking the ORBIT runtime protocol (Phase 3+). \
             Requires an explicit protocol endpoint in runtime config."
            .into(),
        capabilities: RuntimeCapabilities {
            supports_sessions: true,
            supports_resume: false,
            supports_streaming: true,
            supports_cancel: true,
            supports_instances: true,
            supports_tool_events: true,
            supports_usage_reporting: false,
            runtime_managed_providers: false,
            supports_platform_providers: true,
            execution_modes: vec![ExecutionMode::OnDemand, ExecutionMode::LongRunning],
            max_timeout_seconds: 86_400,
            max_memory_mb: 32_768,
        },
        implementation_status: ImplementationStatus::Planned,
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["endpoint"],
            "properties": {
                "endpoint": { "type": "string" },
                "protocol": { "type": "string", "enum": ["http", "grpc"] }
            }
        }),
        docs_url: None,
    })
}

#[async_trait]
impl RuntimeAdapter for DescriptorOnlyAdapter {
    fn kind(&self) -> RuntimeKind {
        self.descriptor.kind
    }

    fn descriptor(&self) -> RuntimeDescriptor {
        self.descriptor.clone()
    }

    async fn validate_config(&self, config: &RuntimeConfig) -> ValidationReport {
        let mut report = ValidationReport::default();
        if config.kind != self.descriptor.kind {
            report.push("type", "runtime kind does not match this adapter");
        }
        report
    }

    async fn health(&self, _config: &RuntimeConfig) -> Result<RuntimeHealth, RuntimeError> {
        Ok(RuntimeHealth::degraded(format!(
            "{} adapter is not implemented in this build",
            self.descriptor.name
        )))
    }

    async fn create_instance(
        &self,
        _request: CreateInstanceRequest,
    ) -> Result<RuntimeInstance, RuntimeError> {
        Err(not_implemented(self.descriptor.kind))
    }

    async fn start_instance(&self, _instance: &RuntimeInstanceRef) -> Result<(), RuntimeError> {
        Err(not_implemented(self.descriptor.kind))
    }

    async fn stop_instance(&self, _instance: &RuntimeInstanceRef) -> Result<(), RuntimeError> {
        Err(not_implemented(self.descriptor.kind))
    }

    async fn delete_instance(&self, _instance: &RuntimeInstanceRef) -> Result<(), RuntimeError> {
        Err(not_implemented(self.descriptor.kind))
    }

    async fn instance_status(
        &self,
        _instance: &RuntimeInstanceRef,
    ) -> Result<RuntimeInstanceStatus, RuntimeError> {
        Err(not_implemented(self.descriptor.kind))
    }

    async fn execute(&self, _request: ExecutionRequest) -> Result<ExecutionOutcome, RuntimeError> {
        Err(not_implemented(self.descriptor.kind))
    }
}

fn not_implemented(kind: RuntimeKind) -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::Unsupported,
        format!(
            "the '{kind}' runtime adapter is registered but not implemented in this build; \
             use the mock runtime or implement the adapter in crates/orbit-runtime"
        ),
    )
}
