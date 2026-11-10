//! Minimal GitHub REST connector invoked only through the ORBIT tool broker.
//!
//! Read operations may work without a credential (public rate limits); write operations
//! require a token. Credentials never leave the worker.

use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct GitHubClientConfig {
    pub base_url: String,
    pub token: Option<String>,
    pub allowed_repositories: Vec<String>,
    pub default_repository: Option<String>,
}

impl GitHubClientConfig {
    pub fn from_connection(config: &Value, token: Option<String>) -> Self {
        let allowed_repositories = config
            .get("allowedRepositories")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            base_url: config
                .get("baseUrl")
                .and_then(Value::as_str)
                .unwrap_or("https://api.github.com")
                .trim_end_matches('/')
                .to_string(),
            token,
            allowed_repositories,
            default_repository: config
                .get("defaultRepository")
                .and_then(Value::as_str)
                .map(str::to_string),
        }
    }

    fn repo(&self, args: &Value) -> Result<(String, String), String> {
        let combined = match (
            args.get("owner").and_then(Value::as_str),
            args.get("repo").and_then(Value::as_str),
        ) {
            (Some(owner), Some(repo)) => format!("{owner}/{repo}"),
            _ => self.default_repository.clone().ok_or_else(|| {
                "owner and repo are required (or set config.defaultRepository)".to_string()
            })?,
        };
        let (owner, repo) = combined
            .split_once('/')
            .ok_or_else(|| "repository must be in owner/repo form".to_string())?;
        validate_segment(owner, "owner")?;
        validate_segment(repo, "repo")?;
        if !self.allowed_repositories.is_empty()
            && !self
                .allowed_repositories
                .contains(&format!("{owner}/{repo}").to_lowercase())
        {
            return Err(format!(
                "repository {owner}/{repo} is not in the connection's allowedRepositories allowlist"
            ));
        }
        Ok((owner.to_string(), repo.to_string()))
    }

    fn requires_token(&self, capability: &str) -> Result<&str, String> {
        self.token.as_deref().ok_or_else(|| {
            format!(
                "GitHub capability '{capability}' requires a credential; add a token to the connection"
            )
        })
    }
}

fn validate_segment(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 100
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(format!("invalid GitHub {field}"));
    }
    Ok(())
}

fn number(args: &Value) -> Result<u64, String> {
    args.get("number")
        .and_then(Value::as_u64)
        .ok_or_else(|| "issue number is required".to_string())
}

pub async fn invoke(
    client: &reqwest::Client,
    config: &GitHubClientConfig,
    capability: &str,
    args: &Value,
) -> Result<Value, String> {
    let (owner, repo) = config.repo(args)?;
    let url = request_url(config, capability, &owner, &repo, args)?;

    let mut request = match capability {
        "issues.list" | "issues.get" => client.get(url),
        "issues.comment" | "issues.create" => client.post(url),
        other => return Err(format!("unsupported GitHub capability: {other}")),
    };
    if let Some(token) = &config.token {
        request = request.bearer_auth(token);
    } else if matches!(capability, "issues.comment" | "issues.create") {
        // Public reads are allowed (anonymous GitHub rate limits); writes are not.
        config.requires_token(capability)?;
    }
    request = request
        .header(reqwest::header::USER_AGENT, "orbit-agent-platform")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    match capability {
        "issues.comment" => {
            let body = args
                .get("body")
                .and_then(Value::as_str)
                .filter(|b| !b.trim().is_empty())
                .ok_or_else(|| "comment body is required".to_string())?;
            request = request.json(&json!({ "body": body }));
        }
        "issues.create" => {
            let title = args
                .get("title")
                .and_then(Value::as_str)
                .filter(|t| !t.trim().is_empty())
                .ok_or_else(|| "issue title is required".to_string())?;
            let mut payload = json!({ "title": title });
            if let Some(body) = args.get("body").and_then(Value::as_str) {
                payload["body"] = json!(body);
            }
            if let Some(labels) = args.get("labels").and_then(Value::as_array) {
                let labels: Vec<String> = labels
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                if !labels.is_empty() {
                    payload["labels"] = json!(labels);
                }
            }
            request = request.json(&payload);
        }
        _ => {}
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("GitHub request failed: {e}"))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .unwrap_or_else(|_| json!({ "message": "empty GitHub response" }));

    if !status.is_success() {
        let message = body
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("GitHub API error");
        return Err(format!("GitHub API returned {status}: {message}"));
    }

    Ok(match capability {
        "issues.list" => {
            let items: Vec<Value> = body
                .as_array()
                .map(|items| items.iter().map(summarize_issue).collect())
                .unwrap_or_default();
            json!({ "repository": format!("{owner}/{repo}"), "count": items.len(), "issues": items })
        }
        _ => summarize_issue(&body),
    })
}

fn request_url(
    config: &GitHubClientConfig,
    capability: &str,
    owner: &str,
    repo: &str,
    args: &Value,
) -> Result<String, String> {
    let base = &config.base_url;
    Ok(match capability {
        "issues.list" => {
            let mut url = format!("{base}/repos/{owner}/{repo}/issues?state=open&per_page=20");
            if let Some(state) = args.get("state").and_then(Value::as_str) {
                if matches!(state, "open" | "closed" | "all") {
                    url = url.replace("state=open", &format!("state={state}"));
                }
            }
            if let Some(labels) = args.get("labels").and_then(Value::as_str) {
                let labels: String = labels
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ',' | '-' | '_' | ' '))
                    .collect();
                if !labels.is_empty() {
                    url.push_str(&format!("&labels={}", labels.replace(' ', "%20")));
                }
            }
            url
        }
        "issues.get" => format!("{base}/repos/{owner}/{repo}/issues/{}", number(args)?),
        "issues.comment" => format!(
            "{base}/repos/{owner}/{repo}/issues/{}/comments",
            number(args)?
        ),
        "issues.create" => format!("{base}/repos/{owner}/{repo}/issues"),
        other => return Err(format!("unsupported GitHub capability: {other}")),
    })
}

fn summarize_issue(issue: &Value) -> Value {
    json!({
        "id": issue.get("id"),
        "number": issue.get("number"),
        "title": issue.get("title"),
        "state": issue.get("state"),
        "body": issue.get("body").and_then(Value::as_str).map(|b| b.chars().take(4000).collect::<String>()),
        "url": issue.get("html_url"),
        "author": issue.get("user").and_then(|u| u.get("login")),
        "labels": issue.get("labels").and_then(Value::as_array).map(|labels| {
            labels.iter().filter_map(|l| l.get("name").and_then(Value::as_str)).collect::<Vec<_>>()
        }).unwrap_or_default(),
        "comments": issue.get("comments"),
        "createdAt": issue.get("created_at"),
        "updatedAt": issue.get("updated_at"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_allowlist_and_validation() {
        let config = GitHubClientConfig {
            base_url: "https://api.github.com".into(),
            token: Some("t".into()),
            allowed_repositories: vec!["acme/orbit".into()],
            default_repository: None,
        };
        assert!(config.repo(&json!({"owner":"acme","repo":"orbit"})).is_ok());
        assert!(config
            .repo(&json!({"owner":"acme","repo":"other"}))
            .is_err());
        assert!(config.repo(&json!({"owner":"../","repo":"orbit"})).is_err());
        assert!(config.repo(&json!({})).is_err());
    }

    #[test]
    fn writes_require_token() {
        let config = GitHubClientConfig {
            base_url: "https://api.github.com".into(),
            token: None,
            allowed_repositories: vec![],
            default_repository: Some("acme/orbit".into()),
        };
        assert!(config.requires_token("issues.comment").is_err());
        // Reads may be anonymous; the write path is what requires a credential.
        assert!(config.token.is_none());
    }

    #[test]
    fn builds_expected_urls() {
        let config = GitHubClientConfig {
            base_url: "https://api.github.com".into(),
            token: None,
            allowed_repositories: vec![],
            default_repository: Some("acme/orbit".into()),
        };
        assert_eq!(
            request_url(
                &config,
                "issues.get",
                "acme",
                "orbit",
                &json!({"number": 42})
            )
            .unwrap(),
            "https://api.github.com/repos/acme/orbit/issues/42"
        );
        assert_eq!(
            request_url(&config, "issues.list", "acme", "orbit", &json!({})).unwrap(),
            "https://api.github.com/repos/acme/orbit/issues?state=open&per_page=20"
        );
    }
}
