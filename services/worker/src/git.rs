//! Git operations for coding agents.
//!
//! Credentials are injected only into the short-lived git child process via
//! `GIT_ASKPASS`; they are not written to the repository remote URL or workspace files.

use std::path::Path;

use serde_json::Value;
use tokio::process::Command;

use crate::client::{WorkerClient, WorkerConnection};

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

fn apply_git_auth(command: &mut Command, askpass: &Path) {
    command
        .env("GIT_ASKPASS", askpass)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_USERNAME", "x-access-token");
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
    if workspace.join(".git").exists() {
        return Ok(Some("repository already checked out".into()));
    }
    if !git_available() {
        anyhow::bail!("repo.checkout requires git in the worker image");
    }
    let connection = client.connection(execution_id, connection_id).await?;
    let Some(repo) = repo_name(&connection, input) else {
        // The binding may be shared by coding and non-coding teams. A GitHub API-only
        // team can list/comment on issues without a local checkout.
        return Ok(Some(
            "repository checkout skipped: no repository configured".into(),
        ));
    };
    validate_repo(&connection, &repo)?;
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
        .unwrap_or("https://github.com")
        .trim_end_matches('/');
    let branch = input_string(input, "branch");
    let mut command = Command::new("git");
    command
        .current_dir(workspace)
        .args(["clone", "--depth", "1"]);
    if let Some(branch) = branch {
        command.args(["--branch", &branch]);
    }
    command.arg(format!("{clone_base}/{repo}.git")).arg(".");
    if let Some(token) = &token {
        command.env("ORBIT_GIT_TOKEN", token);
    }
    if let Some(askpass) = &askpass {
        apply_git_auth(&mut command, askpass);
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
    Ok(Some(format!("checked out {repo}")))
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
    let token = connection
        .credential
        .as_ref()
        .map(|secret| secret.value.clone())
        .ok_or_else(|| anyhow::anyhow!("GitHub connection has no token"))?;
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
    apply_git_auth(&mut command, &askpass);
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
    let token = connection
        .credential
        .as_ref()
        .map(|secret| secret.value.clone())
        .ok_or_else(|| anyhow::anyhow!("GitHub connection has no token"))?;
    let askpass = write_askpass().await?;
    let mut command = Command::new("git");
    command
        .current_dir(workspace)
        .args(["push", "origin", &format!("HEAD:refs/heads/{branch}")]);
    command.env("ORBIT_GIT_TOKEN", &token);
    apply_git_auth(&mut command, &askpass);
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
