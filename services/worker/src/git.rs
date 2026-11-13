//! Git operations for coding agents.
//!
//! Credentials are injected only into the short-lived git child process via
//! `GIT_ASKPASS`; they are not written to the repository remote URL or workspace files.

use std::path::Path;

use serde_json::Value;
use tokio::process::Command;

use crate::client::{WorkerClient, WorkerConnection};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSpec {
    pub connection_id: Option<uuid::Uuid>,
    pub repository: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub branch: Option<String>,
}

pub fn repo_specs(input: &Value) -> Vec<RepoSpec> {
    input
        .get("payload")
        .and_then(|payload| payload.get("repositories"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value::<RepoSpec>(item.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

fn validate_repo_path(path: &str) -> anyhow::Result<()> {
    if path.is_empty() {
        return Ok(());
    }
    if path.starts_with('/') || path.starts_with('\\') || path.contains(":") {
        anyhow::bail!("repository path must be a relative workspace path");
    }
    if path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        anyhow::bail!("repository path cannot escape the workspace");
    }
    Ok(())
}

fn git_available() -> bool {
    which("git")
}

fn which(binary: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|path| path.join(binary).is_file()))
        .unwrap_or(false)
}

async fn write_askpass() -> anyhow::Result<std::path::PathBuf> {
    let path = std::env::temp_dir().join(format!(".orbit-git-askpass-{}.sh", uuid::Uuid::now_v7()));
    tokio::fs::write(
        &path,
        "#!/bin/sh\ncase \"$1\" in\n  Username*) printf '%s' \"${ORBIT_GIT_USERNAME:-x-access-token}\" ;;\n  Password*) printf '%s' \"$ORBIT_GIT_TOKEN\" ;;\nesac\n",
    )
    .await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = tokio::fs::metadata(&path).await?.permissions();
        permissions.set_mode(0o700);
        tokio::fs::set_permissions(&path, permissions).await?;
    }
    Ok(path)
}

fn apply_git_auth(command: &mut Command, askpass: &Path, username: &str) {
    command
        .env("GIT_ASKPASS", askpass)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("ORBIT_GIT_USERNAME", username);
}

fn git_username(connection: &WorkerConnection) -> String {
    connection
        .connection
        .config
        .get("gitUsername")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            if connection.connector.slug == "gitlab" {
                "oauth2".into()
            } else {
                "x-access-token".into()
            }
        })
}

fn input_string(input: &Value, key: &str) -> Option<String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            input
                .get("payload")
                .and_then(|payload| payload.get(key))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn repo_name(connection: &WorkerConnection, input: &Value) -> Option<String> {
    input_string(input, "repository").or_else(|| {
        connection
            .connection
            .config
            .get("defaultRepository")
            .or_else(|| connection.connection.config.get("defaultProject"))
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn validate_repo(connection: &WorkerConnection, repo: &str) -> anyhow::Result<()> {
    let ok = repo
        .split_once('/')
        .map(|(owner, name)| {
            !owner.is_empty()
                && !name.is_empty()
                && owner
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        })
        .unwrap_or(false);
    if !ok {
        anyhow::bail!("repository must be owner/repo");
    }
    if let Some(allowed) = connection
        .connection
        .config
        .get("allowedRepositories")
        .or_else(|| connection.connection.config.get("allowedProjects"))
        .and_then(Value::as_array)
    {
        if !allowed.is_empty()
            && !allowed
                .iter()
                .filter_map(Value::as_str)
                .any(|value| value.eq_ignore_ascii_case(repo))
        {
            anyhow::bail!("repository {repo} is not in the connection's allowedRepositories");
        }
    }
    Ok(())
}

/// Clone the GitHub repository into the workspace before the harness starts.
///
/// Single-repository compatibility path. Multi-repository teams should use
/// `checkout_workspace`, which reads the execution's repository list.
pub async fn checkout(
    client: &WorkerClient,
    execution_id: uuid::Uuid,
    connection_id: Option<uuid::Uuid>,
    input: &Value,
    workspace: &Path,
) -> anyhow::Result<Option<String>> {
    let Some(connection_id) = connection_id else {
        return Ok(None);
    };
    let connection = client.connection(execution_id, connection_id).await?;
    let Some(repo) = repo_name(&connection, input) else {
        return Ok(Some(
            "repository checkout skipped: no repository configured".into(),
        ));
    };
    checkout_repository(
        client,
        execution_id,
        connection_id,
        &repo,
        input_string(input, "branch").as_deref(),
        workspace,
    )
    .await
    .map(Some)
}

/// Check out the repository list from the execution payload sequentially.
///
/// A single repository preserves the current root layout for compatibility; multiple
/// repositories are cloned into their declared workspace subdirectories.
pub async fn checkout_workspace(
    client: &WorkerClient,
    execution_id: uuid::Uuid,
    default_connection_id: Option<uuid::Uuid>,
    input: &Value,
    workspace: &Path,
) -> anyhow::Result<Vec<String>> {
    let specs = repo_specs(input);
    if specs.is_empty() {
        if let Some(note) = checkout(
            client,
            execution_id,
            default_connection_id,
            input,
            workspace,
        )
        .await?
        {
            return Ok(vec![note]);
        }
        return Ok(Vec::new());
    }

    let single = specs.len() == 1;
    let mut notes = Vec::new();
    for spec in &specs {
        validate_repo_path(&spec.path)?;
        let target = if single && spec.path.is_empty() {
            workspace.to_path_buf()
        } else if spec.path.is_empty() {
            workspace.join(spec.repository.split('/').next_back().unwrap_or("repo"))
        } else {
            workspace.join(&spec.path)
        };
        let connection_id = spec
            .connection_id
            .or(default_connection_id)
            .ok_or_else(|| anyhow::anyhow!("repository '{}' has no connection", spec.repository))?;
        notes.push(
            checkout_repository(
                client,
                execution_id,
                connection_id,
                &spec.repository,
                spec.branch.as_deref(),
                &target,
            )
            .await?,
        );
    }
    Ok(notes)
}

async fn checkout_repository(
    client: &WorkerClient,
    execution_id: uuid::Uuid,
    connection_id: uuid::Uuid,
    repo: &str,
    branch: Option<&str>,
    workspace: &Path,
) -> anyhow::Result<String> {
    if workspace.join(".git").exists() {
        return Ok(format!("repository {repo} already checked out"));
    }
    if !git_available() {
        anyhow::bail!("repo.checkout requires git in the worker image");
    }
    let connection = client.connection(execution_id, connection_id).await?;
    validate_repo(&connection, repo)?;
    let token = connection
        .credential
        .as_ref()
        .map(|secret| secret.value.clone());
    let askpass = match &token {
        Some(_) => Some(write_askpass().await?),
        None => None,
    };
    let clone_base = connection
        .connection
        .config
        .get("cloneBaseUrl")
        .and_then(Value::as_str)
        .unwrap_or_else(|| {
            if connection.connector.slug == "gitlab" {
                "https://gitlab.com"
            } else {
                "https://github.com"
            }
        })
        .trim_end_matches('/');
    let username = git_username(&connection);
    tokio::fs::create_dir_all(workspace).await?;
    let mut command = Command::new("git");
    command
        .current_dir(workspace)
        .args(["clone", "--depth", "1"]);
    if let Some(branch) = branch.filter(|branch| !branch.is_empty()) {
        command.args(["--branch", branch]);
    }
    command.arg(format!("{clone_base}/{repo}.git")).arg(".");
    if let Some(token) = &token {
        command.env("ORBIT_GIT_TOKEN", token);
    }
    if let Some(askpass) = &askpass {
        apply_git_auth(&mut command, askpass, &username);
    }
    let output = command.output().await?;
    if let Some(askpass) = &askpass {
        let _ = tokio::fs::remove_file(askpass).await;
    }
    if !output.status.success() {
        anyhow::bail!(
            "git clone failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(format!("checked out {repo}"))
}

/// Commit all workspace changes. Returns the commit subject/status.
pub async fn commit(
    client: &WorkerClient,
    execution_id: uuid::Uuid,
    connection_id: uuid::Uuid,
    workspace: &Path,
    message: &str,
) -> anyhow::Result<Value> {
    let connection = client.connection(execution_id, connection_id).await?;
    let username = git_username(&connection);
    let token = connection
        .credential
        .as_ref()
        .map(|secret| secret.value.clone())
        .ok_or_else(|| anyhow::anyhow!("connection has no token"))?;
    let askpass = write_askpass().await?;
    let add = Command::new("git")
        .current_dir(workspace)
        .args(["add", "-A"])
        .output()
        .await?;
    if !add.status.success() {
        let _ = tokio::fs::remove_file(&askpass).await;
        anyhow::bail!("git add failed: {}", String::from_utf8_lossy(&add.stderr));
    }
    let mut command = Command::new("git");
    command.current_dir(workspace).args([
        "-c",
        "user.name=ORBIT Agent",
        "-c",
        "user.email=orbit@localhost",
        "commit",
        "-m",
        message,
    ]);
    command.env("ORBIT_GIT_TOKEN", &token);
    apply_git_auth(&mut command, &askpass, &username);
    let output = command.output().await?;
    let _ = tokio::fs::remove_file(&askpass).await;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("nothing to commit") {
            return Ok(serde_json::json!({ "status": "no_changes" }));
        }
        anyhow::bail!("git commit failed: {}", stderr.trim());
    }
    Ok(serde_json::json!({
        "status": "committed",
        "message": message,
        "output": String::from_utf8_lossy(&output.stdout).trim(),
    }))
}

/// Push the current HEAD to a branch.
pub async fn push(
    client: &WorkerClient,
    execution_id: uuid::Uuid,
    connection_id: uuid::Uuid,
    workspace: &Path,
    branch: &str,
) -> anyhow::Result<Value> {
    if branch.is_empty()
        || branch.len() > 200
        || branch
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, ':' | '~' | '^' | '?' | '*' | '[' | '\\'))
    {
        anyhow::bail!("invalid branch name");
    }
    let connection = client.connection(execution_id, connection_id).await?;
    let username = git_username(&connection);
    let token = connection
        .credential
        .as_ref()
        .map(|secret| secret.value.clone())
        .ok_or_else(|| anyhow::anyhow!("connection has no token"))?;
    let askpass = write_askpass().await?;
    let mut command = Command::new("git");
    command
        .current_dir(workspace)
        .args(["push", "origin", &format!("HEAD:refs/heads/{branch}")]);
    command.env("ORBIT_GIT_TOKEN", &token);
    apply_git_auth(&mut command, &askpass, &username);
    let output = command.output().await?;
    let _ = tokio::fs::remove_file(&askpass).await;
    if !output.status.success() {
        anyhow::bail!(
            "git push failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(serde_json::json!({ "status": "pushed", "branch": branch }))
}
