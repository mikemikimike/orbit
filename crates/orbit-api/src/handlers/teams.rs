//! Agent teams: sequential role handoffs materialized as normal executions.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Extension;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use orbit_domain::{HarnessKind, TriggerSource, ValidationIssue};
use orbit_security::{AuthContext, Permission};
use orbit_store::{AgentTeam, TeamRun, TeamStep};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamStepInput {
    pub agent_id: Uuid,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// `pipeline` runs each agent once in order. `native` runs the primary agent only
    /// and materializes the remaining agents as harness-native subagents.
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Shared goal/instruction for the team. The first step receives it as its task;
    /// later steps receive it together with the previous step output.
    #[serde(default)]
    pub instructions: String,
    /// Structured workspace repository (owner/repo). Instructions alone cannot drive
    /// repository checkout; this is passed to the worker.
    #[serde(default)]
    pub repository: Option<String>,
    /// Optional base/working branch for repository checkout.
    #[serde(default)]
    pub branch: Option<String>,
    /// Worker profile used for automatic launch.
    #[serde(default)]
    pub worker_profile_id: Option<Uuid>,
    #[serde(default)]
    pub auto_launch: bool,
    /// Skills materialized for every step in the team.
    #[serde(default)]
    pub skills: Vec<String>,
    pub steps: Vec<TeamStepInput>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamRunRequest {
    /// Optional run-specific task. Falls back to the team instruction.
    #[serde(default)]
    pub task: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Explicitly allow bound writes for this run without per-call approvals (L4).
    #[serde(default)]
    pub auto_approve: bool,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TeamRunDetail {
    pub run: TeamRun,
    pub steps: Vec<TeamStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamDefinition {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub worker_profile_id: Option<Uuid>,
    #[serde(default)]
    pub auto_launch: bool,
    #[serde(default)]
    pub skills: Vec<String>,
    pub steps: Vec<TeamStepDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamStepDefinition {
    pub agent_id: Uuid,
}

fn default_mode() -> String {
    "pipeline".into()
}

pub(crate) fn parse_definition(team: &AgentTeam) -> Result<TeamDefinition, ApiError> {
    serde_json::from_value(team.definition.clone())
        .map_err(|error| ApiError::internal(format!("team definition is invalid: {error}")))
}

async fn validate_native_primary(
    state: &AppState,
    org_id: Uuid,
    mode: &str,
    steps: &[TeamStepInput],
    issues: &mut Vec<ValidationIssue>,
) {
    if mode != "native" {
        return;
    }
    let Some(primary) = steps.first() else {
        return;
    };
    let Ok(agent) = state.store.get_agent(org_id, primary.agent_id).await else {
        return;
    };
    let Ok(version) = state.store.get_current_version(org_id, &agent).await else {
        return;
    };
    match serde_json::from_value::<orbit_domain::AgentSpec>(version.spec) {
        Ok(spec) if spec.harness.kind == HarnessKind::OpenCode => {}
        Ok(_) => issues.push(ValidationIssue::new(
            "mode",
            "native Agent Teams currently require an OpenCode primary agent; Pi and other harnesses can use pipeline mode",
        )),
        Err(error) => issues.push(ValidationIssue::new(
            "mode",
            format!("primary agent spec is invalid: {error}"),
        )),
    }
}

pub async fn create_team(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<CreateTeamRequest>,
) -> Result<(StatusCode, Json<AgentTeam>), ApiError> {
    auth.require(Permission::SettingsWrite)?;
    if body.name.trim().is_empty() || body.steps.is_empty() {
        return Err(ApiError::validation(vec![
            ValidationIssue::new("name", "name is required"),
            ValidationIssue::new("steps", "at least one step is required"),
        ]));
    }
    if !matches!(body.mode.as_str(), "pipeline" | "native") {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "mode",
            "mode must be pipeline or native",
        )]));
    }
    if let Some(repository) = body
        .repository
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
    {
        if !repository.contains('/') || repository.chars().any(char::is_whitespace) {
            return Err(ApiError::validation(vec![ValidationIssue::new(
                "repository",
                "repository must be owner/repo",
            )]));
        }
    }
    let mut issues = Vec::new();
    let mut agent_ids = std::collections::BTreeSet::new();
    for step in &body.steps {
        if !agent_ids.insert(step.agent_id) {
            issues.push(ValidationIssue::new(
                "steps.agentId",
                "the same agent cannot appear twice in one team",
            ));
        }
        if state
            .store
            .get_agent(auth.org_id, step.agent_id)
            .await
            .is_err()
        {
            issues.push(ValidationIssue::new(
                "steps.agentId",
                format!("unknown agent '{}'", step.agent_id),
            ));
        }
    }
    for slug in &body.skills {
        if state
            .store
            .get_skill_by_slug(auth.org_id, slug)
            .await?
            .is_none()
        {
            issues.push(ValidationIssue::new(
                "skills",
                format!("unknown skill '{slug}'"),
            ));
        }
    }
    validate_native_primary(&state, auth.org_id, &body.mode, &body.steps, &mut issues).await;
    if !issues.is_empty() {
        return Err(ApiError::validation(issues));
    }
    let definition = serde_json::json!({
        "mode": body.mode,
        "instructions": body.instructions.trim(),
        "repository": body.repository.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "branch": body.branch.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "workerProfileId": body.worker_profile_id,
        "autoLaunch": body.auto_launch,
        "skills": body.skills,
        "steps": body.steps.iter().map(|step| serde_json::json!({
            "agentId": step.agent_id,
        })).collect::<Vec<_>>()
    });
    let team = state
        .store
        .create_agent_team(
            auth.org_id,
            body.name.trim(),
            body.description.trim(),
            &definition,
            Some(auth.user_id),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(team)))
}

pub async fn update_team(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(team_id): Path<Uuid>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<CreateTeamRequest>,
) -> Result<Json<AgentTeam>, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    if body.name.trim().is_empty() || body.steps.is_empty() {
        return Err(ApiError::validation(vec![
            ValidationIssue::new("name", "name is required"),
            ValidationIssue::new("steps", "at least one step is required"),
        ]));
    }
    if !matches!(body.mode.as_str(), "pipeline" | "native") {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "mode",
            "mode must be pipeline or native",
        )]));
    }
    if let Some(repository) = body
        .repository
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty())
    {
        if !repository.contains('/') || repository.chars().any(char::is_whitespace) {
            return Err(ApiError::validation(vec![ValidationIssue::new(
                "repository",
                "repository must be owner/repo",
            )]));
        }
    }
    let mut issues = Vec::new();
    let mut agent_ids = std::collections::BTreeSet::new();
    for step in &body.steps {
        if !agent_ids.insert(step.agent_id) {
            issues.push(ValidationIssue::new(
                "steps.agentId",
                "the same agent cannot appear twice in one team",
            ));
        }
        if state
            .store
            .get_agent(auth.org_id, step.agent_id)
            .await
            .is_err()
        {
            issues.push(ValidationIssue::new(
                "steps.agentId",
                format!("unknown agent '{}'", step.agent_id),
            ));
        }
    }
    for slug in &body.skills {
        if state
            .store
            .get_skill_by_slug(auth.org_id, slug)
            .await?
            .is_none()
        {
            issues.push(ValidationIssue::new(
                "skills",
                format!("unknown skill '{slug}'"),
            ));
        }
    }
    validate_native_primary(&state, auth.org_id, &body.mode, &body.steps, &mut issues).await;
    if !issues.is_empty() {
        return Err(ApiError::validation(issues));
    }
    let definition = serde_json::json!({
        "mode": body.mode,
        "instructions": body.instructions.trim(),
        "repository": body.repository.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "branch": body.branch.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "workerProfileId": body.worker_profile_id,
        "autoLaunch": body.auto_launch,
        "skills": body.skills,
        "steps": body.steps.iter().map(|step| serde_json::json!({
            "agentId": step.agent_id,
        })).collect::<Vec<_>>()
    });
    Ok(Json(
        state
            .store
            .update_agent_team(
                auth.org_id,
                team_id,
                body.name.trim(),
                body.description.trim(),
                &definition,
            )
            .await?,
    ))
}

pub async fn list_teams(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<AgentTeam>>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    Ok(Json(state.store.list_agent_teams(auth.org_id).await?))
}

pub async fn get_team(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(team_id): Path<Uuid>,
) -> Result<Json<AgentTeam>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    Ok(Json(
        state.store.get_agent_team(auth.org_id, team_id).await?,
    ))
}

pub async fn delete_team(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(team_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    auth.require(Permission::SettingsWrite)?;
    state.store.delete_agent_team(auth.org_id, team_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn native_subagents(
    state: &AppState,
    org_id: Uuid,
    definition: &TeamDefinition,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let mut subagents = Vec::new();
    for step in definition.steps.iter().skip(1) {
        let agent = state.store.get_agent(org_id, step.agent_id).await?;
        let version = state.store.get_current_version(org_id, &agent).await?;
        let spec: orbit_domain::AgentSpec =
            serde_json::from_value(version.spec.clone()).map_err(|error| {
                ApiError::internal(format!("stored agent spec is invalid: {error}"))
            })?;
        let mut instructions = spec.instructions.system.clone();
        if !spec.instructions.constraints.is_empty() {
            instructions.push_str("\n\nConstraints:\n");
            for constraint in &spec.instructions.constraints {
                instructions.push_str(&format!("- {constraint}\n"));
            }
        }
        let model = match (&spec.model.provider, &spec.model.model) {
            (Some(provider), Some(model)) if !model.contains('/') => {
                Some(format!("{provider}/{model}"))
            }
            (_, Some(model)) => Some(model.clone()),
            _ => None,
        };
        subagents.push(serde_json::json!({
            "name": agent.name,
            "slug": agent.slug,
            "description": agent.description,
            "instructions": instructions,
            "model": model,
        }));
    }
    Ok(subagents)
}

pub(crate) async fn start_step(
    state: &AppState,
    run: &TeamRun,
    definition: &TeamDefinition,
    step_index: usize,
    previous_output: Option<&serde_json::Value>,
) -> Result<TeamStep, ApiError> {
    let step = definition
        .steps
        .get(step_index)
        .ok_or_else(|| ApiError::internal("team step index out of range"))?;
    let agent = state.store.get_agent(run.org_id, step.agent_id).await?;
    let version = state.store.get_current_version(run.org_id, &agent).await?;
    let spec: orbit_domain::AgentSpec = serde_json::from_value(version.spec.clone())
        .map_err(|error| ApiError::internal(format!("stored agent spec is invalid: {error}")))?;
    let task = run
        .input
        .get("task")
        .and_then(|value| value.as_str())
        .unwrap_or("Team run");
    let instructions = run
        .input
        .get("instructions")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let previous = previous_output
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".into());
    let task_text = if previous_output.is_none() {
        format!("Team goal: {instructions}\n\n{task}")
    } else {
        format!(
            "Team goal: {instructions}\n\nRole: {}\n\nOriginal task: {task}\n\nPrevious step output:\n{previous}",
            agent.name
        )
    };
    let native = definition.mode == "native";
    if native && spec.harness.kind != HarnessKind::OpenCode {
        return Err(ApiError::conflict(
            "native Agent Teams currently require an OpenCode primary agent",
        ));
    }
    let mut team_payload = serde_json::json!({
        "runId": run.id,
        "mode": definition.mode,
        "goal": instructions,
        "agent": agent.name,
        "previous": previous,
    });
    if native {
        team_payload["subagents"] =
            serde_json::Value::Array(native_subagents(state, run.org_id, definition).await?);
    }
    let input = serde_json::json!({
        "task": task_text,
        "payload": {
            "teamRunId": run.id,
            "stepIndex": step_index,
            "agentName": agent.name,
            "previous": previous_output,
            "repository": run.input.get("payload").and_then(|value| value.get("repository")).cloned(),
            "branch": run.input.get("payload").and_then(|value| value.get("branch")).cloned(),
        },
        "provenance": {
            "source": "team",
            "trusted": true,
            "subject": run.id,
            "receivedAt": chrono::Utc::now(),
        },
        "team": team_payload,
        "autonomyOverride": if run.input.get("autoApprove").and_then(|value| value.as_bool()) == Some(true) { 4 } else { 0 }
    });
    let execution = state
        .store
        .create_execution(orbit_store::executions::NewExecution {
            org_id: run.org_id,
            agent_id: agent.id,
            agent_version_id: version.id,
            agent_name: &agent.name,
            harness_kind: spec.harness.kind,
            trigger_source: TriggerSource::Manual,
            placement: spec.harness.placement.as_str(),
            trigger_id: None,
            input,
            skills: &definition.skills,
            timeout_seconds: spec.resources.timeout_seconds,
            requested_by: run.requested_by,
            idempotency_key: Some(&format!("team:{}:{}", run.id, step_index)),
            team_run_id: Some(run.id),
            team_step_index: Some(step_index as i32),
        })
        .await?;
    let created = state
        .store
        .create_team_step(
            run.id,
            run.org_id,
            step_index as i32,
            &agent.name,
            agent.id,
            version.id,
            &definition.skills,
            &execution.input,
            Some(execution.id),
        )
        .await?;
    Ok(created)
}

pub async fn create_run(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(team_id): Path<Uuid>,
    crate::extract::ApiJson(body): crate::extract::ApiJson<CreateTeamRunRequest>,
) -> Result<(StatusCode, Json<TeamRunDetail>), ApiError> {
    auth.require(Permission::ExecutionsCreate)?;
    let team = state.store.get_agent_team(auth.org_id, team_id).await?;
    let definition = parse_definition(&team)?;
    let task = if body.task.trim().is_empty() {
        definition.instructions.trim()
    } else {
        body.task.trim()
    };
    if task.is_empty() {
        return Err(ApiError::validation(vec![ValidationIssue::new(
            "task",
            "task or team instructions are required",
        )]));
    }
    let mut payload = body.payload.clone();
    if !payload.is_object() {
        payload = serde_json::json!({});
    }
    if payload
        .get("repository")
        .and_then(|value| value.as_str())
        .is_none()
    {
        if let Some(repository) = &definition.repository {
            payload["repository"] = serde_json::json!(repository);
        }
    }
    if payload
        .get("branch")
        .and_then(|value| value.as_str())
        .is_none()
    {
        if let Some(branch) = &definition.branch {
            payload["branch"] = serde_json::json!(branch);
        }
    }
    let input = serde_json::json!({
        "task": task,
        "instructions": definition.instructions,
        "payload": payload,
        "definition": definition,
        "autoApprove": body.auto_approve,
    });
    let run = state
        .store
        .create_team_run(auth.org_id, team.id, &input, Some(auth.user_id))
        .await?;
    let step = match start_step(&state, &run, &definition, 0, None).await {
        Ok(step) => step,
        Err(error) => {
            let _ = state
                .store
                .update_team_run(run.id, "failed", None, Some(&error.message))
                .await;
            return Err(error);
        }
    };
    if definition.auto_launch {
        if let (Some(profile_id), Some(execution_id)) =
            (definition.worker_profile_id, step.execution_id)
        {
            crate::handlers::worker_launches::launch_profile_internal(
                &state,
                &auth,
                profile_id,
                Some(execution_id),
            )
            .await?;
        }
    }
    Ok((
        StatusCode::CREATED,
        Json(TeamRunDetail {
            run,
            steps: vec![step],
        }),
    ))
}

pub async fn list_runs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<TeamRun>>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    Ok(Json(state.store.list_team_runs(auth.org_id).await?))
}

pub async fn get_run(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<TeamRunDetail>, ApiError> {
    auth.require(Permission::HarnessesRead)?;
    let run = state.store.get_team_run(auth.org_id, run_id).await?;
    let steps = state.store.list_team_steps(run.id).await?;
    Ok(Json(TeamRunDetail { run, steps }))
}

/// Advance a running team after its current execution reaches a terminal state.
pub(crate) async fn advance(state: &AppState, run: TeamRun) -> anyhow::Result<()> {
    let Some(current) = state.store.current_team_step(run.id).await? else {
        return Ok(());
    };
    if current.status != "running" {
        return Ok(());
    }
    let Some(execution_id) = current.execution_id else {
        return Ok(());
    };
    let execution = state.store.get_execution_by_id(execution_id).await?;
    if !matches!(
        orbit_domain::ExecutionStatus::parse(&execution.status),
        Some(status) if status.is_terminal()
    ) {
        return Ok(());
    }
    let definition = match run.input.get("definition").cloned() {
        Some(value) => serde_json::from_value::<TeamDefinition>(value)
            .map_err(|error| anyhow::anyhow!("team run definition is invalid: {error}"))?,
        None => {
            let team = state.store.get_agent_team(run.org_id, run.team_id).await?;
            parse_definition(&team).map_err(|error| anyhow::anyhow!(error.message))?
        }
    };
    if execution.status == "succeeded" {
        state
            .store
            .finish_team_step(current.id, "succeeded", execution.output.as_ref(), None)
            .await?;
        if definition.mode == "native" {
            state
                .store
                .update_team_run(run.id, "succeeded", execution.output.as_ref(), None)
                .await?;
            let _ = state
                .store
                .set_team_queue_status_by_run(run.id, "succeeded", None)
                .await;
            return Ok(());
        }
        let next = (current.step_index as usize) + 1;
        if next < definition.steps.len() {
            let run_for_step = state.store.get_team_run(run.org_id, run.id).await?;
            let created = start_step(
                state,
                &run_for_step,
                &definition,
                next,
                execution.output.as_ref(),
            )
            .await
            .map_err(|error| anyhow::anyhow!(error.message))?;
            if definition.auto_launch {
                if let (Some(profile_id), Some(execution_id)) =
                    (definition.worker_profile_id, created.execution_id)
                {
                    let auth = AuthContext {
                        user_id: run.requested_by.unwrap_or(Uuid::nil()),
                        org_id: run.org_id,
                        email: String::new(),
                        display_name: "Agent Team".into(),
                        role: orbit_domain::UserRole::Operator,
                        method: orbit_security::AuthMethod::ApiToken,
                        scopes: vec![],
                        token_id: None,
                    };
                    crate::handlers::worker_launches::launch_profile_internal(
                        state,
                        &auth,
                        profile_id,
                        Some(execution_id),
                    )
                    .await
                    .map_err(|error| anyhow::anyhow!(error.message))?;
                }
            }
        } else {
            state
                .store
                .update_team_run(run.id, "succeeded", execution.output.as_ref(), None)
                .await?;
        }
    } else {
        state
            .store
            .finish_team_step(
                current.id,
                "failed",
                execution.output.as_ref(),
                Some(&execution.status),
            )
            .await?;
        state
            .store
            .update_team_run(
                run.id,
                "failed",
                execution.output.as_ref(),
                Some(&format!(
                    "step '{}' finished with {}",
                    current.name, execution.status
                )),
            )
            .await?;
    }
    Ok(())
}
