//! Execution-scoped, policy-enforcing tool broker.
//!
//! The agent harness talks MCP to `orbit-mcp-broker`, which forwards tool calls to this
//! loopback HTTP broker. The broker validates the execution token, enforces autonomy and
//! capability allowlists, pauses for approvals, leases credentials, invokes connectors and
//! writes audit/execution events. The harness never receives provider credentials.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use orbit_domain::{
    evaluate_capability, AgentSpec, ApprovalMode, AutonomyLevel, CapabilityEffect,
    GrantedCapability, LogLevel, PolicyDecision,
};
use orbit_store::{Approval, Connection, ExecutionRecord};

use crate::client::WorkerClient;
use crate::github::{self, GitHubClientConfig};
use crate::gitlab::{self, GitLabClientConfig};

#[derive(Debug, Clone)]
struct ToolSpec {
    capability: String,
    name: String,
    description: String,
    effect: CapabilityEffect,
    input_schema: Value,
}

struct BrokerState {
    client: WorkerClient,
    execution: ExecutionRecord,
    workspace: std::path::PathBuf,
    autonomy: AutonomyLevel,
    approval_mode: ApprovalMode,
    connection: Connection,
    connector_slug: String,
    connector_client: ConnectorClient,
    tools: Vec<ToolSpec>,
    token: String,
    http: reqwest::Client,
}

enum ConnectorClient {
    Github(GitHubClientConfig),
    Gitlab(GitLabClientConfig),
}

pub struct BrokerHandle {
    pub url: String,
    pub token: String,
    pub tool_count: usize,
    task: tokio::task::JoinHandle<()>,
}

impl BrokerHandle {
    pub fn mcp_command(&self) -> Vec<String> {
        vec![std::env::var("ORBIT_MCP_BROKER_BIN")
            .unwrap_or_else(|_| "/usr/local/bin/orbit-mcp-broker".to_string())]
    }
}

impl Drop for BrokerHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn github_tool_specs(
    capabilities: &[String],
    effects: &std::collections::BTreeMap<String, CapabilityEffect>,
) -> Vec<ToolSpec> {
    capabilities
        .iter()
        .map(|capability| {
            let tool = format!("github_{}", capability.replace('.', "_"));
            let effect = effects
                .get(capability)
                .copied()
                .unwrap_or(CapabilityEffect::Read);
            let input_schema = match capability.as_str() {
                "issues.list" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" },
                        "repo": { "type": "string" },
                        "state": { "type": "string", "enum": ["open", "closed", "all"] },
                        "labels": { "type": "string", "description": "Comma-separated labels" }
                    }
                }),
                "issues.get" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" }, "repo": { "type": "string" },
                        "number": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["number"]
                }),
                "issues.comment" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" }, "repo": { "type": "string" },
                        "number": { "type": "integer", "minimum": 1 },
                        "body": { "type": "string", "minLength": 1, "maxLength": 65536 }
                    },
                    "required": ["number", "body"]
                }),
                "issues.create" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" }, "repo": { "type": "string" },
                        "title": { "type": "string", "minLength": 1, "maxLength": 256 },
                        "body": { "type": "string", "maxLength": 65536 },
                        "labels": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["title"]
                }),
                "issues.update" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" }, "repo": { "type": "string" },
                        "number": { "type": "integer", "minimum": 1 },
                        "title": { "type": "string" },
                        "body": { "type": "string" },
                        "state": { "type": "string", "enum": ["open", "closed"] },
                        "labels": { "type": "array", "items": { "type": "string" } },
                        "assignees": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["number"]
                }),
                "pulls.list" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" }, "repo": { "type": "string" },
                        "state": { "type": "string", "enum": ["open", "closed", "all"] }
                    }
                }),
                "pulls.get" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" }, "repo": { "type": "string" },
                        "number": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["number"]
                }),
                "pulls.create" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" }, "repo": { "type": "string" },
                        "title": { "type": "string", "minLength": 1, "maxLength": 256 },
                        "head": { "type": "string", "description": "Source branch" },
                        "base": { "type": "string", "default": "main" },
                        "body": { "type": "string" },
                        "draft": { "type": "boolean" }
                    },
                    "required": ["title", "head"]
                }),
                "checks.list" => json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string" }, "repo": { "type": "string" },
                        "ref": { "type": "string", "description": "Commit SHA or branch" }
                    },
                    "required": ["ref"]
                }),
                "repo.commit" => json!({
                    "type": "object",
                    "properties": {
                        "message": { "type": "string", "minLength": 1, "maxLength": 2000 }
                    },
                    "required": ["message"]
                }),
                "repo.push" => json!({
                    "type": "object",
                    "properties": {
                        "branch": { "type": "string", "minLength": 1, "maxLength": 200 }
                    },
                    "required": ["branch"]
                }),
                _ => json!({ "type": "object", "properties": {} }),
            };
            let description = match capability.as_str() {
                "issues.list" => "List open issues in a GitHub repository (read-only).",
                "issues.get" => "Read one GitHub issue (read-only).",
                "issues.comment" => {
                    "Post a comment on a GitHub issue. Write action, may require human approval."
                }
                "issues.create" => {
                    "Create a GitHub issue. Write action, may require human approval."
                }
                "issues.update" => {
                    "Update a GitHub issue, including state, labels and assignees. Write action."
                }
                "pulls.list" => "List pull requests in a repository (read-only).",
                "pulls.get" => "Read one pull request (read-only).",
                "pulls.create" => {
                    "Create a pull request from a branch. Write action, may require human approval."
                }
                "checks.list" => "Read CI check runs for a commit or branch (read-only).",
                "repo.commit" => "Commit all workspace changes. Write action.",
                "repo.push" => "Push the current workspace HEAD to a branch. External side effect.",
                _ => "GitHub capability.",
            };
            ToolSpec {
                capability: capability.clone(),
                name: tool,
                description: description.to_string(),
                effect,
                input_schema,
            }
        })
        .collect()
}

fn gitlab_tool_specs(
    capabilities: &[String],
    effects: &std::collections::BTreeMap<String, CapabilityEffect>,
) -> Vec<ToolSpec> {
    capabilities
        .iter()
        .map(|capability| {
            let tool = format!("gitlab_{}", capability.replace('.', "_"));
            let effect = effects
                .get(capability)
                .copied()
                .unwrap_or(CapabilityEffect::Read);
            let input_schema = match capability.as_str() {
                "projects.list" => json!({ "type": "object", "properties": {} }),
                "group_issues.list" => json!({
                    "type": "object",
                    "properties": {
                        "group": { "type": "string" },
                        "state": { "type": "string" },
                        "labels": { "type": "string" },
                        "search": { "type": "string" }
                    }
                }),
                "boards.list" => json!({
                    "type": "object",
                    "properties": { "group": { "type": "string" } }
                }),
                "issues.list" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "state": { "type": "string" },
                        "labels": { "type": "string" },
                        "search": { "type": "string" }
                    }
                }),
                "issues.get" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "iid": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["iid"]
                }),
                "issues.create" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "labels": { "type": "array", "items": { "type": "string" } },
                        "confidential": { "type": "boolean" }
                    },
                    "required": ["title"]
                }),
                "issues.update" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "iid": { "type": "integer", "minimum": 1 },
                        "title": { "type": "string" },
                        "description": { "type": "string" },
                        "labels": { "type": "array", "items": { "type": "string" } },
                        "state_event": { "type": "string", "enum": ["close", "reopen"] },
                        "assigneeIds": { "type": "array", "items": { "type": "integer" } }
                    },
                    "required": ["iid"]
                }),
                "issues.comment" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "iid": { "type": "integer", "minimum": 1 },
                        "body": { "type": "string" }
                    },
                    "required": ["iid", "body"]
                }),
                "merge_requests.list" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "state": { "type": "string" }
                    }
                }),
                "merge_requests.get" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "iid": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["iid"]
                }),
                "merge_requests.create" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "title": { "type": "string" },
                        "sourceBranch": { "type": "string" },
                        "targetBranch": { "type": "string" },
                        "description": { "type": "string" },
                        "draft": { "type": "boolean" }
                    },
                    "required": ["title", "sourceBranch"]
                }),
                "merge_requests.comment" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "iid": { "type": "integer", "minimum": 1 },
                        "body": { "type": "string" }
                    },
                    "required": ["iid", "body"]
                }),
                "merge_requests.approve" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "iid": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["iid"]
                }),
                "merge_requests.merge" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "iid": { "type": "integer", "minimum": 1 },
                        "sha": { "type": "string" },
                        "mergeCommitMessage": { "type": "string" }
                    },
                    "required": ["iid"]
                }),
                "pipelines.status" => json!({
                    "type": "object",
                    "properties": {
                        "project": { "type": "string" },
                        "ref": { "type": "string" }
                    }
                }),
                "repo.commit" => json!({
                    "type": "object",
                    "properties": { "message": { "type": "string" } },
                    "required": ["message"]
                }),
                "repo.push" => json!({
                    "type": "object",
                    "properties": { "branch": { "type": "string" } },
                    "required": ["branch"]
                }),
                _ => json!({ "type": "object", "properties": {} }),
            };
            let description = match capability.as_str() {
                "projects.list" => "List accessible GitLab projects.",
                "group_issues.list" => "List issues across a GitLab group or group board.",
                "boards.list" => "List issue boards for a GitLab group.",
                "issues.list" => "List issues in a GitLab project.",
                "issues.get" => "Read one GitLab issue.",
                "issues.create" => "Create a GitLab issue.",
                "issues.update" => "Update a GitLab issue, labels or state.",
                "issues.comment" => "Post a note on a GitLab issue.",
                "merge_requests.list" => "List GitLab merge requests.",
                "merge_requests.get" => "Read one GitLab merge request.",
                "merge_requests.create" => "Create a GitLab merge request.",
                "merge_requests.comment" => "Post a note on a GitLab merge request.",
                "merge_requests.approve" => "Approve a GitLab merge request.",
                "merge_requests.merge" => "Merge a GitLab merge request.",
                "pipelines.status" => "Read the latest GitLab pipeline status.",
                "repo.commit" => "Commit workspace changes.",
                "repo.push" => "Push the workspace branch to GitLab.",
                _ => "GitLab capability.",
            };
            ToolSpec {
                capability: capability.clone(),
                name: tool,
                description: description.to_string(),
                effect,
                input_schema,
            }
        })
        .collect()
}

/// Start the broker for one execution. Returns `None` when the agent has no GitHub binding.
pub async fn start(
    client: WorkerClient,
    spec: &AgentSpec,
    execution: &ExecutionRecord,
    workspace: &std::path::Path,
) -> anyhow::Result<Option<BrokerHandle>> {
    let Some(binding) = spec.connectors.iter().find(|b| {
        matches!(b.connector.as_str(), "github" | "gitlab") && !b.capabilities.is_empty()
    }) else {
        return Ok(None);
    };
    let connector_slug = binding.connector.clone();
    let Some(connection_id) = binding.connection_id else {
        anyhow::bail!(
            "Connector '{connector_slug}' is bound without a connection; select a connection in Agent Studio"
        );
    };
    let worker_connection = client.connection(execution.id, connection_id).await?;
    let connection = worker_connection.connection;
    let connector = worker_connection.connector;
    if connection.status != "active" {
        anyhow::bail!("{connector_slug} connection is not active");
    }

    let mut effects = std::collections::BTreeMap::new();
    if let Some(items) = connector.capabilities.as_array() {
        for item in items {
            if let (Some(id), Some(effect)) = (
                item.get("id").and_then(Value::as_str),
                item.get("effect").and_then(Value::as_str),
            ) {
                if let Some(effect) = CapabilityEffect::parse(effect) {
                    effects.insert(id.to_string(), effect);
                }
            }
        }
    }
    // Only expose capabilities that exist in the connector manifest.
    let allowed: Vec<String> = binding
        .capabilities
        .iter()
        .filter(|capability| effects.contains_key(*capability) && *capability != "repo.checkout")
        .cloned()
        .collect();
    if allowed.is_empty() {
        // A binding may only request pre-execution capabilities such as repo.checkout.
        return Ok(None);
    }
    let tools = if connector_slug == "gitlab" {
        gitlab_tool_specs(&allowed, &effects)
    } else {
        github_tool_specs(&allowed, &effects)
    };

    let token = worker_connection
        .credential
        .as_ref()
        .map(|credential| credential.value.clone());
    let connector_client = if connector_slug == "gitlab" {
        ConnectorClient::Gitlab(GitLabClientConfig::from_connection(
            &connection.config,
            token,
        ))
    } else {
        ConnectorClient::Github(GitHubClientConfig::from_connection(
            &connection.config,
            token,
        ))
    };

    let autonomy = AutonomyLevel::from_level(spec.autonomy.level).unwrap_or(AutonomyLevel::OBSERVE);
    let state = Arc::new(BrokerState {
        client,
        execution: execution.clone(),
        workspace: workspace.to_path_buf(),
        autonomy,
        approval_mode: spec.autonomy.approval_mode,
        connection,
        connector_slug,
        connector_client,
        tools,
        token: format!("{}{}", Uuid::now_v7().simple(), Uuid::now_v7().simple()),
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?,
    });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = Router::new()
        .route("/tools", get(list_tools))
        .route("/tools/call", post(call_tool))
        .with_state(state.clone());
    let task = tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            tracing::error!(%error, "tool broker server stopped");
        }
    });
    tracing::info!(
        execution_id = %state.execution.id,
        tools = state.tools.len(),
        "tool broker started on loopback"
    );
    Ok(Some(BrokerHandle {
        url: format!("http://{address}"),
        token: state.token.clone(),
        tool_count: state.tools.len(),
        task,
    }))
}

async fn list_tools(
    State(state): State<Arc<BrokerState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorized(&headers, &state.token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
            .into_response();
    }
    let tools: Vec<Value> = state
        .tools
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool.input_schema,
            })
        })
        .collect();
    (StatusCode::OK, Json(json!({ "tools": tools }))).into_response()
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    name: String,
    #[serde(default)]
    arguments: Value,
}

async fn call_tool(
    State(state): State<Arc<BrokerState>>,
    headers: HeaderMap,
    Json(call): Json<ToolCall>,
) -> impl IntoResponse {
    if !authorized(&headers, &state.token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
            .into_response();
    }
    let Some(tool) = state.tools.iter().find(|tool| tool.name == call.name) else {
        record_denied(&state, &call.name, "unknown or not allowed tool").await;
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": format!("tool '{}' is not allowed", call.name) })),
        )
            .into_response();
    };

    let grants: Vec<GrantedCapability> = state
        .tools
        .iter()
        .map(|tool| GrantedCapability {
            connection_id: state.connection.id,
            connector: state.connector_slug.clone(),
            capability: tool.capability.clone(),
            effect: tool.effect,
        })
        .collect();
    let decision = evaluate_capability(
        state.autonomy,
        state.approval_mode,
        &tool.capability,
        tool.effect,
        &grants,
    );

    let mut approval_id: Option<Uuid> = None;
    if decision == PolicyDecision::Deny {
        record_denied(
            &state,
            &tool.capability,
            "denied by autonomy/capability policy",
        )
        .await;
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": format!("capability '{}' is not allowed by policy", tool.capability) })),
        )
            .into_response();
    }
    if decision == PolicyDecision::RequireApproval {
        match request_approval(&state, tool, &call.arguments).await {
            Ok(approval) => approval_id = Some(approval.id),
            Err(error) => {
                return tool_error(&state, tool, &call.arguments, &error.to_string()).await;
            }
        }
    }

    let result = if matches!(tool.capability.as_str(), "repo.commit" | "repo.push") {
        let outcome = if tool.capability == "repo.commit" {
            let message = call
                .arguments
                .get("message")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty());
            match message {
                Some(message) => {
                    crate::git::commit(
                        &state.client,
                        state.execution.id,
                        state.connection.id,
                        &state.workspace,
                        message,
                    )
                    .await
                }
                None => Err(anyhow::anyhow!("commit message is required")),
            }
        } else {
            let branch = call.arguments.get("branch").and_then(Value::as_str);
            match branch {
                Some(branch) => {
                    crate::git::push(
                        &state.client,
                        state.execution.id,
                        state.connection.id,
                        &state.workspace,
                        branch,
                    )
                    .await
                }
                None => Err(anyhow::anyhow!("branch is required")),
            }
        };
        outcome.map_err(|error| error.to_string())
    } else {
        match &state.connector_client {
            ConnectorClient::Github(config) => {
                github::invoke(&state.http, config, &tool.capability, &call.arguments).await
            }
            ConnectorClient::Gitlab(config) => {
                gitlab::invoke(&state.http, config, &tool.capability, &call.arguments).await
            }
        }
    };

    match result {
        Ok(result) => {
            record_event(
                &state,
                "tool_call",
                LogLevel::Info,
                &format!(
                    "{} {}{}",
                    tool.capability,
                    if decision == PolicyDecision::RequireApproval {
                        "executed after approval"
                    } else {
                        "executed"
                    },
                    format_approval(approval_id)
                ),
                json!({
                    "capability": tool.capability,
                    "effect": tool.effect.as_str(),
                    "decision": decision.as_str(),
                    "approvalId": approval_id,
                    "arguments": truncate_json(&call.arguments, 4000),
                    "result": truncate_json(&result, 8000),
                }),
            )
            .await;
            record_audit(
                &state,
                "tool.call.executed",
                json!({
                    "capability": tool.capability,
                    "decision": decision.as_str(),
                    "approvalId": approval_id,
                }),
            )
            .await;
            mcp_text(result, false)
        }
        Err(message) => tool_error(&state, tool, &call.arguments, &message).await,
    }
}

async fn tool_error(
    state: &BrokerState,
    tool: &ToolSpec,
    arguments: &Value,
    message: &str,
) -> axum::response::Response {
    record_event(
        state,
        "tool_call",
        LogLevel::Error,
        &format!("{} failed: {}", tool.capability, truncate(message, 300)),
        json!({
            "capability": tool.capability,
            "error": message,
            "arguments": truncate_json(arguments, 2000),
        }),
    )
    .await;
    record_audit(
        state,
        "tool.call.failed",
        json!({ "capability": tool.capability, "error": truncate(message, 500) }),
    )
    .await;
    mcp_text(json!({ "error": message }), true)
}

async fn request_approval(
    state: &BrokerState,
    tool: &ToolSpec,
    arguments: &Value,
) -> anyhow::Result<Approval> {
    let wait_seconds = approval_wait_seconds(&state.execution);
    let approval = state
        .client
        .create_approval(
            state.execution.id,
            json!({
                "capability": tool.capability,
                "connector": state.connector_slug,
                "connectionId": state.connection.id,
                "connectionName": state.connection.name,
                "effect": tool.effect.as_str(),
                "agentName": state.execution.agent_name,
                "arguments": truncate_json(arguments, 4000),
            }),
            wait_seconds,
        )
        .await?;
    record_event(
        state,
        "tool_call",
        LogLevel::Warn,
        &format!(
            "approval requested for {} (approval {})",
            tool.capability, approval.id
        ),
        json!({
            "capability": tool.capability,
            "effect": tool.effect.as_str(),
            "approvalId": approval.id,
            "status": "pending",
            "arguments": truncate_json(arguments, 4000),
        }),
    )
    .await;
    record_audit(
        state,
        "tool.call.approval_requested",
        json!({ "capability": tool.capability, "approvalId": approval.id }),
    )
    .await;

    let decided = state
        .client
        .await_approval(state.execution.id, approval.id, wait_seconds)
        .await?;
    record_event(
        state,
        "tool_call",
        if decided.status == "approved" {
            LogLevel::Info
        } else {
            LogLevel::Warn
        },
        &format!("approval {} for {}", decided.status, tool.capability),
        json!({ "approvalId": decided.id, "status": decided.status }),
    )
    .await;
    if decided.status != "approved" {
        anyhow::bail!(
            "capability '{}' requires approval; request {}",
            tool.capability,
            decided.status
        );
    }
    record_audit(
        state,
        "tool.call.approved",
        json!({ "capability": tool.capability, "approvalId": decided.id }),
    )
    .await;
    Ok(decided)
}

fn approval_wait_seconds(execution: &ExecutionRecord) -> u64 {
    let started = execution.started_at.unwrap_or(execution.created_at);
    let deadline = started + chrono::Duration::seconds(execution.timeout_seconds as i64);
    let remaining = (deadline - Utc::now()).num_seconds().max(5) as u64;
    remaining.saturating_sub(3).clamp(2, 900)
}

async fn record_denied(state: &BrokerState, capability: &str, reason: &str) {
    record_event(
        state,
        "tool_call",
        LogLevel::Warn,
        &format!("denied tool call {capability}: {reason}"),
        json!({ "capability": capability, "reason": reason, "decision": "deny" }),
    )
    .await;
    record_audit(
        state,
        "tool.call.denied",
        json!({ "capability": capability, "reason": reason }),
    )
    .await;
}

async fn record_event(
    state: &BrokerState,
    kind: &str,
    level: LogLevel,
    message: &str,
    data: Value,
) {
    let _ = state
        .client
        .event(
            state.execution.id,
            &crate::client::EventRequest {
                kind,
                level: level.as_str(),
                message,
                data,
            },
        )
        .await;
}

async fn record_audit(state: &BrokerState, action: &str, metadata: Value) {
    if let Err(error) = state
        .client
        .audit(
            state.execution.id,
            action,
            Some(state.connection.id),
            metadata,
        )
        .await
    {
        tracing::warn!(%error, "failed to write tool audit log");
    }
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| value == token)
        .unwrap_or(false)
}

fn mcp_text(value: Value, is_error: bool) -> axum::response::Response {
    let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into());
    (
        StatusCode::OK,
        Json(json!({
            "content": [{ "type": "text", "text": text }],
            "isError": is_error,
        })),
    )
        .into_response()
}

fn format_approval(approval_id: Option<Uuid>) -> String {
    approval_id
        .map(|id| format!(" (approval {id})"))
        .unwrap_or_default()
}

fn truncate(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        value.to_string()
    } else {
        let mut end = limit;
        while end > 0 && !value.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &value[..end])
    }
}

fn truncate_json(value: &Value, limit: usize) -> Value {
    match serde_json::to_string(value) {
        Ok(serialized) if serialized.len() <= limit => value.clone(),
        Ok(serialized) => json!(truncate(&serialized, limit)),
        Err(_) => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_names_are_stable_and_mcp_friendly() {
        let effects = std::collections::BTreeMap::from([
            ("issues.list".to_string(), CapabilityEffect::Read),
            ("issues.comment".to_string(), CapabilityEffect::Write),
        ]);
        let tools = github_tool_specs(
            &["issues.list".to_string(), "issues.comment".to_string()],
            &effects,
        );
        assert_eq!(tools[0].name, "github_issues_list");
        assert_eq!(tools[1].name, "github_issues_comment");
        assert_eq!(tools[1].effect, CapabilityEffect::Write);
    }

    #[test]
    fn approval_wait_is_bounded_by_execution_timeout() {
        let mut execution = ExecutionRecord {
            id: Uuid::nil(),
            org_id: Uuid::nil(),
            agent_id: Uuid::nil(),
            agent_version_id: Uuid::nil(),
            agent_name: "test".into(),
            harness_kind: "opencode".into(),
            trigger_id: None,
            trigger_source: "manual".into(),
            placement: "auto".into(),
            status: "running".into(),
            session_ref: None,
            input: Value::Null,
            skills: vec![],
            output: None,
            error: None,
            usage: json!({}),
            timeout_seconds: 60,
            idempotency_key: None,
            requested_by: None,
            worker_id: None,
            team_run_id: None,
            team_step_index: None,
            created_at: Utc::now(),
            started_at: Some(Utc::now()),
            finished_at: None,
            updated_at: Utc::now(),
        };
        assert!(approval_wait_seconds(&execution) <= 60);
        execution.timeout_seconds = 3600;
        assert!(approval_wait_seconds(&execution) <= 900);
    }
}
