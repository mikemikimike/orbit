use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;

use orbit_contracts::{
    CreateInstanceRequest, ExecutionOutcome, ExecutionRequest, RuntimeAdapter, RuntimeCapabilities,
    RuntimeConfig, RuntimeDescriptor, RuntimeError, RuntimeErrorKind, RuntimeHealth,
    RuntimeInstance, RuntimeInstanceRef, RuntimeInstanceStatus,
};
use orbit_domain::{ExecutionMode, ImplementationStatus, RuntimeKind, ValidationReport};

/// OpenCode runtime adapter.
///
/// **Status: scaffolded.** This adapter implements the descriptor, capability model,
/// configuration validation and CLI preflight. Real task execution (session creation,
/// `opencode run --format json`, event mapping, cancellation) is delivered in Phase 1 and
/// must be verified against an installed OpenCode version before it is claimed as
/// working. OpenCode Go is a *provider* concept handled through `runtime-managed`
/// provider configuration, not a separate runtime.
#[derive(Debug, Clone, Default)]
pub struct OpenCodeRuntimeAdapter;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct OpenCodeSettings {
    /// CLI binary name or absolute path. Defaults to `opencode`.
    pub binary: Option<String>,
    /// Working directory (relative to the execution workspace).
    pub workspace_subdir: Option<String>,
    /// Runtime-managed provider name (e.g. `opencode-go`). Not validated against a
    /// hardcoded inventory: arbitrary provider names are accepted.
    pub provider: Option<String>,
    pub model: Option<String>,
    pub agent: Option<String>,
    /// Additional CLI arguments, passed verbatim after validation.
    pub extra_args: Vec<String>,
    /// `--auto` for non-interactive execution in isolated environments.
    pub auto_approve: bool,
    /// OpenCode server URL, when using a long-running `opencode serve` instance.
    pub server_url: Option<String>,
}

impl OpenCodeSettings {
    pub fn from_config(config: &RuntimeConfig) -> Result<Self, RuntimeError> {
        if config.kind != RuntimeKind::OpenCode {
            return Err(RuntimeError::new(
                RuntimeErrorKind::InvalidConfig,
                "OpenCode adapter received a non-opencode runtime configuration",
            ));
        }
        if config.settings.is_null() {
            return Ok(Self::default());
        }
        serde_json::from_value(config.settings.clone()).map_err(|e| {
            RuntimeError::new(
                RuntimeErrorKind::InvalidConfig,
                format!("invalid OpenCode runtime config: {e}"),
            )
        })
    }

    pub fn binary(&self) -> &str {
        self.binary.as_deref().unwrap_or("opencode")
    }

    pub fn cli_args(&self, prompt: &str) -> Vec<String> {
        let mut args = vec![
            "run".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        if self.auto_approve {
            args.push("--auto".to_string());
        }
        if let Some(agent) = &self.agent {
            args.push("--agent".to_string());
            args.push(agent.clone());
        }
        if let (Some(provider), Some(model)) = (&self.provider, &self.model) {
            args.push("--model".to_string());
            args.push(format!("{provider}/{model}"));
        }
        args.extend(self.extra_args.iter().cloned());
        args.push("--".to_string());
        args.push(prompt.to_string());
        args
    }
}

impl OpenCodeRuntimeAdapter {
    pub fn descriptor() -> RuntimeDescriptor {
        RuntimeDescriptor {
            kind: RuntimeKind::OpenCode,
            name: "OpenCode".into(),
            description:
                "OpenCode CLI runtime with sessions, streaming output and runtime-managed providers."
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
                supports_platform_providers: true,
                execution_modes: vec![ExecutionMode::OnDemand],
                max_timeout_seconds: 86_400,
                max_memory_mb: 16_384,
            },
            implementation_status: ImplementationStatus::Scaffolded,
            config_schema: serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "binary": { "type": "string", "description": "opencode binary (default: opencode)" },
                    "workspaceSubdir": { "type": "string" },
                    "provider": { "type": "string", "description": "runtime-managed provider, e.g. opencode-go" },
                    "model": { "type": "string" },
                    "agent": { "type": "string" },
                    "extraArgs": { "type": "array", "items": { "type": "string" }, "maxItems": 32 },
                    "autoApprove": { "type": "boolean", "default": false },
                    "serverUrl": { "type": "string", "description": "opencode serve URL (long-running mode)" }
                }
            }),
            docs_url: Some("https://opencode.ai/docs".into()),
        }
    }

    /// Resolve the installed CLI and return its version string.
    pub async fn probe(&self, settings: &OpenCodeSettings) -> Result<String, RuntimeError> {
        let candidate = settings.binary();
        let mut command = tokio::process::Command::new(candidate);
        command.arg("--version");
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        let output = tokio::time::timeout(Duration::from_secs(8), command.output())
            .await
            .map_err(|_| RuntimeError::unavailable("opencode --version timed out"))?
            .map_err(|e| {
                RuntimeError::new(
                    RuntimeErrorKind::Unavailable,
                    format!("opencode binary '{candidate}' is not available: {e}"),
                )
            })?;
        if !output.status.success() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::Unavailable,
                format!(
                    "opencode --version exited with {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl RuntimeAdapter for OpenCodeRuntimeAdapter {
    fn kind(&self) -> RuntimeKind {
        RuntimeKind::OpenCode
    }

    fn descriptor(&self) -> RuntimeDescriptor {
        Self::descriptor()
    }

    async fn validate_config(&self, config: &RuntimeConfig) -> ValidationReport {
        let mut report = ValidationReport::default();
        match OpenCodeSettings::from_config(config) {
            Ok(settings) => {
                if settings
                    .binary
                    .as_deref()
                    .is_some_and(|b| b.trim().is_empty())
                {
                    report.push("binary", "must not be empty");
                }
                if settings.extra_args.len() > 32 {
                    report.push("extraArgs", "at most 32 additional arguments");
                }
                for (idx, arg) in settings.extra_args.iter().enumerate() {
                    if arg.chars().count() > 256 {
                        report.push(
                            format!("extraArgs[{idx}]"),
                            "argument must be <= 256 characters",
                        );
                    }
                }
                if let Some(url) = &settings.server_url {
                    if !(url.starts_with("http://") || url.starts_with("https://")) {
                        report.push("serverUrl", "must be an http(s) URL");
                    }
                }
            }
            Err(err) => report.push("config", err.message),
        }
        report
    }

    async fn health(&self, config: &RuntimeConfig) -> Result<RuntimeHealth, RuntimeError> {
        let settings = OpenCodeSettings::from_config(config)?;
        match self.probe(&settings).await {
            Ok(version) => {
                let mut health = RuntimeHealth::healthy();
                health
                    .details
                    .insert("version".into(), version.trim().to_string());
                health
                    .details
                    .insert("binary".into(), settings.binary().to_string());
                Ok(health)
            }
            Err(err) => Ok(RuntimeHealth::degraded(err.message)),
        }
    }

    async fn create_instance(
        &self,
        _request: CreateInstanceRequest,
    ) -> Result<RuntimeInstance, RuntimeError> {
        Err(RuntimeError::unsupported(
            "long-running instances (Phase 3). Use executionMode=on_demand.",
        ))
    }

    async fn start_instance(&self, _instance: &RuntimeInstanceRef) -> Result<(), RuntimeError> {
        Err(RuntimeError::unsupported("long-running instances"))
    }

    async fn stop_instance(&self, _instance: &RuntimeInstanceRef) -> Result<(), RuntimeError> {
        Err(RuntimeError::unsupported("long-running instances"))
    }

    async fn delete_instance(&self, _instance: &RuntimeInstanceRef) -> Result<(), RuntimeError> {
        Err(RuntimeError::unsupported("long-running instances"))
    }

    async fn instance_status(
        &self,
        _instance: &RuntimeInstanceRef,
    ) -> Result<RuntimeInstanceStatus, RuntimeError> {
        Err(RuntimeError::unsupported("long-running instances"))
    }

    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionOutcome, RuntimeError> {
        let settings = OpenCodeSettings::from_config(&RuntimeConfig {
            kind: RuntimeKind::OpenCode,
            settings: request.runtime_config.clone(),
        })?;

        // Fail fast with an actionable message if the CLI is missing. This is real
        // preflight work; task execution itself is delivered in Phase 1.
        let version = self.probe(&settings).await?;
        Err(RuntimeError::new(
            RuntimeErrorKind::Unsupported,
            format!(
                "OpenCode execution is scaffolded in this build (detected {version}). \
                 Phase 1 wires session creation and `opencode run --format json` event mapping; \
                 see docs/architecture/runtime-contract.md for the implementation plan."
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_cli_arguments_without_hardcoded_providers() {
        let settings = OpenCodeSettings {
            provider: Some("any-provider".into()),
            model: Some("any-model".into()),
            auto_approve: true,
            ..Default::default()
        };
        let args = settings.cli_args("do the thing");
        assert!(args.contains(&"any-provider/any-model".to_string()));
        assert!(args.contains(&"--auto".to_string()));
        assert_eq!(args.last().unwrap(), "do the thing");
    }

    #[test]
    fn validation_rejects_bad_url() {
        let config = RuntimeConfig {
            kind: RuntimeKind::OpenCode,
            settings: serde_json::json!({ "serverUrl": "not-a-url" }),
        };
        let report = futures::executor::block_on(OpenCodeRuntimeAdapter.validate_config(&config));
        assert!(!report.is_valid());
    }
}
