//! Optional worker launcher.
//!
//! Receives already-authorized launch requests from `orbit-api` and creates Docker
//! containers or Kubernetes Pods. It is the only component with Docker socket or
//! Kubernetes Workload RBAC; the API never receives those privileges.

use std::collections::BTreeMap;
use std::process::Stdio;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    token: String,
    docker: String,
    kubectl: String,
    namespace: String,
    opencode_auth_export: Option<String>,
    pi_agent_dir: Option<String>,
    image_pull_secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchRequest {
    launch_id: Uuid,
    profile_id: Uuid,
    name: String,
    harness_kind: String,
    target: String,
    execution_driver: String,
    image: String,
    image_tag: String,
    auth_secret_name: Option<String>,
    #[serde(default)]
    env: Value,
    #[serde(default)]
    resources: Value,
    api_url: String,
    worker_token: String,
    worker_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchStatus {
    status: String,
    runtime_id: Option<String>,
    worker_id: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Logs {
    logs: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let state = AppState {
        token: std::env::var("ORBIT_LAUNCHER_TOKEN")
            .map_err(|_| anyhow::anyhow!("ORBIT_LAUNCHER_TOKEN is required"))?,
        docker: std::env::var("ORBIT_LAUNCHER_DOCKER").unwrap_or_else(|_| "docker".into()),
        kubectl: std::env::var("ORBIT_LAUNCHER_KUBECTL").unwrap_or_else(|_| "kubectl".into()),
        namespace: std::env::var("ORBIT_LAUNCHER_NAMESPACE").unwrap_or_else(|_| "orbit".into()),
        opencode_auth_export: std::env::var("ORBIT_OPENCODE_AUTH_EXPORT")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        pi_agent_dir: std::env::var("ORBIT_PI_AGENT_DIR")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        image_pull_secret: std::env::var("ORBIT_LAUNCHER_IMAGE_PULL_SECRET")
            .ok()
            .filter(|value| !value.trim().is_empty()),
    };
    let addr = std::env::var("ORBIT_LAUNCHER_ADDR").unwrap_or_else(|_| "0.0.0.0:8090".into());
    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/launches", post(launch))
        .route("/launches/{launch_id}", get(status))
        .route("/launches/{launch_id}/stop", post(stop))
        .route("/launches/{launch_id}/logs", get(logs))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "ORBIT launcher listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> impl IntoResponse {
    StatusCode::OK
}

fn authorized(headers: &HeaderMap, token: &str) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| value == token)
        .unwrap_or(false)
}

fn pod_name(launch_id: Uuid) -> String {
    format!("orbit-worker-{}", &launch_id.to_string()[..8])
}

fn rewrite_api(url: &str) -> String {
    url.replace("//localhost:", "//host.docker.internal:")
        .replace("//127.0.0.1:", "//host.docker.internal:")
}

async fn launch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<LaunchRequest>,
) -> Result<Json<LaunchStatus>, (StatusCode, Json<Value>)> {
    if !authorized(&headers, &state.token) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        ));
    }
    let result = match request.target.as_str() {
        "docker" | "local" => launch_docker(&state, &request).await,
        "kubernetes" => launch_kubernetes(&state, &request).await,
        other => Err(anyhow::anyhow!("unsupported launch target '{other}'")),
    };
    match result {
        Ok(status) => Ok(Json(status)),
        Err(error) => {
            tracing::error!(launch_id = %request.launch_id, %error, "launch failed");
            Err((
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": error.to_string() })),
            ))
        }
    }
}

async fn launch_docker(state: &AppState, request: &LaunchRequest) -> anyhow::Result<LaunchStatus> {
    let name = pod_name(request.launch_id);
    let _ = Command::new(&state.docker)
        .args(["rm", "-f", &name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    let api_url = rewrite_api(&request.api_url);
    let mut args: Vec<String> = vec![
        "run".into(),
        "-d".into(),
        "--name".into(),
        name.clone(),
        "--label".into(),
        format!("orbit.launch-id={}", request.launch_id),
        "--add-host".into(),
        "host.docker.internal:host-gateway".into(),
        "--env".into(),
        format!("ORBIT_API_URL={api_url}"),
        "--env".into(),
        format!("ORBIT_WORKER_TOKEN={}", request.worker_token),
        "--env".into(),
        format!("ORBIT_WORKER_PROFILE_ID={}", request.profile_id),
        "--env".into(),
        format!("ORBIT_WORKER_HARNESSES={}", request.harness_kind),
        "--env".into(),
        format!("ORBIT_WORKER_TARGET={}", request.target),
        "--env".into(),
        format!("ORBIT_EXECUTION_DRIVER={}", request.execution_driver),
        "--env".into(),
        format!("ORBIT_WORKER_ID={}", request.worker_id),
        "--env".into(),
        "ORBIT_WORKSPACE_ROOT=/tmp/orbit-workspaces".into(),
        "--env".into(),
        format!(
            "ORBIT_LOG_FORMAT={}",
            std::env::var("ORBIT_LOG_FORMAT").unwrap_or_else(|_| "text".into())
        ),
    ];
    if let Ok(env) = serde_json::from_value::<BTreeMap<String, String>>(request.env.clone()) {
        for (key, value) in env {
            args.push("--env".into());
            args.push(format!("{key}={value}"));
        }
    }
    let command = "mkdir -p \"$HOME/.local/share/opencode\"; \
         if command -v opencode >/dev/null 2>&1 && [ -f /seed/auth-export.json ]; then \
           opencode auth import --standalone /seed/auth-export.json || true; \
         fi; exec /usr/local/bin/orbit-worker";
    if request.harness_kind == "opencode" {
        if let Some(path) = &state.opencode_auth_export {
            if std::path::Path::new(path).exists() {
                args.push("-v".into());
                args.push(format!("{path}:/seed/auth-export.json:ro"));
            }
        }
    }
    if request.harness_kind == "pi" {
        if let Some(path) = &state.pi_agent_dir {
            if std::path::Path::new(path).exists() {
                args.push("-v".into());
                args.push(format!("{path}:/home/orbit/.pi/agent"));
            }
        }
    }
    args.push("--entrypoint".into());
    args.push("/bin/sh".into());
    args.push(format!("{}:{}", request.image, request.image_tag));
    args.push("-c".into());
    args.push(command.to_string());
    let output = Command::new(&state.docker).args(&args).output().await?;
    if !output.status.success() {
        anyhow::bail!(
            "docker run failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    tracing::info!(launch_id = %request.launch_id, profile = %request.name, container = %name, "worker container started");
    Ok(LaunchStatus {
        status: "running".into(),
        runtime_id: Some(name),
        worker_id: Some(request.worker_id.clone()),
        error: None,
    })
}

async fn docker_exists(state: &AppState, name: &str) -> bool {
    Command::new(&state.docker)
        .args(["inspect", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn docker_status(state: &AppState, name: &str) -> LaunchStatus {
    let output = match Command::new(&state.docker)
        .args([
            "inspect",
            "-f",
            "{{.State.Status}}|{{if .State.Error}}{{.State.Error}}{{end}}",
            name,
        ])
        .output()
        .await
    {
        Ok(output) => output,
        Err(error) => {
            return LaunchStatus {
                status: "failed".into(),
                runtime_id: Some(name.into()),
                worker_id: None,
                error: Some(error.to_string()),
            }
        }
    };
    if !output.status.success() {
        return LaunchStatus {
            status: "stopped".into(),
            runtime_id: Some(name.into()),
            worker_id: None,
            error: None,
        };
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (state_value, error) = stdout.trim().split_once('|').unwrap_or((stdout.trim(), ""));
    let status = match state_value {
        "running" => "running",
        "created" | "restarting" => "starting",
        "exited" | "removing" => "stopped",
        "dead" | "paused" => "failed",
        _ => "starting",
    };
    LaunchStatus {
        status: status.into(),
        runtime_id: Some(name.into()),
        worker_id: None,
        error: (!error.is_empty()).then(|| error.to_string()),
    }
}

async fn launch_kubernetes(
    state: &AppState,
    request: &LaunchRequest,
) -> anyhow::Result<LaunchStatus> {
    let name = pod_name(request.launch_id);
    let api_url = request.api_url.clone();
    let mut env = vec![
        json!({ "name": "ORBIT_API_URL", "value": api_url }),
        json!({ "name": "ORBIT_WORKER_TOKEN", "value": request.worker_token }),
        json!({ "name": "ORBIT_WORKER_PROFILE_ID", "value": request.profile_id.to_string() }),
        json!({ "name": "ORBIT_WORKER_HARNESSES", "value": request.harness_kind }),
        json!({ "name": "ORBIT_WORKER_TARGET", "value": request.target }),
        json!({ "name": "ORBIT_EXECUTION_DRIVER", "value": request.execution_driver }),
        json!({ "name": "ORBIT_WORKER_ID", "value": request.worker_id }),
        json!({ "name": "ORBIT_WORKSPACE_ROOT", "value": "/tmp/orbit-workspaces" }),
    ];
    if let Ok(extra) = serde_json::from_value::<BTreeMap<String, String>>(request.env.clone()) {
        for (key, value) in extra {
            env.push(json!({ "name": key, "value": value }));
        }
    }
    let mut command = vec!["/usr/local/bin/orbit-worker".to_string()];
    let mut mounts = vec![json!({ "name": "tmp", "mountPath": "/tmp" })];
    let mut pod_volumes = vec![json!({ "name": "tmp", "emptyDir": {} })];
    if request.harness_kind == "opencode" {
        if let Some(secret) = &request.auth_secret_name {
            pod_volumes.push(json!({
                "name": "harness-auth",
                "secret": { "secretName": secret, "optional": true }
            }));
            mounts.push(json!({
                "name": "harness-auth",
                "mountPath": "/seed/auth-export.json",
                "subPath": "auth-export.json",
                "readOnly": true
            }));
            command = vec![
                "/bin/sh".into(),
                "-c".into(),
                "mkdir -p \"$HOME/.local/share/opencode\"; if command -v opencode >/dev/null 2>&1 && [ -f /seed/auth-export.json ]; then opencode auth import --standalone /seed/auth-export.json || true; fi; exec /usr/local/bin/orbit-worker".into(),
            ];
        }
    } else if request.harness_kind == "pi" {
        if let Some(secret) = &request.auth_secret_name {
            pod_volumes.push(json!({
                "name": "harness-auth",
                "secret": { "secretName": secret, "optional": true }
            }));
            mounts.push(json!({
                "name": "harness-auth",
                "mountPath": "/home/orbit/.pi/agent"
            }));
        }
    }
    if let Some(size) = request
        .resources
        .get("workspace")
        .and_then(|value| value.get("sizeLimit"))
    {
        pod_volumes[0] = json!({ "name": "tmp", "emptyDir": { "sizeLimit": size } });
    }
    let mut spec = json!({
        "restartPolicy": "Never",
        "containers": [{
            "name": "worker",
            "image": format!("{}:{}", request.image, request.image_tag),
            "imagePullPolicy": "IfNotPresent",
            "env": env,
            "command": command,
            "volumeMounts": mounts
        }],
        "volumes": pod_volumes
    });
    if let Some(secret) = &state.image_pull_secret {
        spec["imagePullSecrets"] = json!([{ "name": secret }]);
    }
    let manifest = json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": name,
            "namespace": state.namespace,
            "labels": { "app.kubernetes.io/name": "orbit-worker", "orbit.launch-id": request.launch_id.to_string() }
        },
        "spec": spec
    });
    let _ = Command::new(&state.kubectl)
        .args([
            "-n",
            &state.namespace,
            "delete",
            "pod",
            &name,
            "--ignore-not-found",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    let mut child = Command::new(&state.kubectl)
        .args(["-n", &state.namespace, "create", "-f", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(manifest.to_string().as_bytes()).await?;
    }
    let output = child.wait_with_output().await?;
    if !output.status.success() {
        anyhow::bail!(
            "kubectl create failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    tracing::info!(launch_id = %request.launch_id, pod = %name, "worker pod created");
    Ok(LaunchStatus {
        status: "starting".into(),
        runtime_id: Some(name),
        worker_id: Some(request.worker_id.clone()),
        error: None,
    })
}

async fn kubernetes_status(state: &AppState, name: &str) -> LaunchStatus {
    let output = Command::new(&state.kubectl)
        .args([
            "-n",
            &state.namespace,
            "get",
            "pod",
            name,
            "-o",
            "jsonpath={.status.phase}",
        ])
        .output()
        .await;
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return LaunchStatus {
                status: "failed".into(),
                runtime_id: Some(name.into()),
                worker_id: None,
                error: Some(error.to_string()),
            }
        }
    };
    if !output.status.success() {
        return LaunchStatus {
            status: "stopped".into(),
            runtime_id: Some(name.into()),
            worker_id: None,
            error: None,
        };
    }
    let phase = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let status = match phase.as_str() {
        "Running" => "running",
        "Pending" | "ContainerCreating" => "starting",
        "Succeeded" => "stopped",
        "Failed" => "failed",
        _ => "starting",
    };
    LaunchStatus {
        status: status.into(),
        runtime_id: Some(name.into()),
        worker_id: None,
        error: None,
    }
}

async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(launch_id): Path<Uuid>,
) -> Result<Json<LaunchStatus>, (StatusCode, Json<Value>)> {
    if !authorized(&headers, &state.token) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        ));
    }
    let name = pod_name(launch_id);
    let status = if docker_exists(&state, &name).await {
        docker_status(&state, &name).await
    } else {
        kubernetes_status(&state, &name).await
    };
    Ok(Json(status))
}

impl AppState {
    async fn kubectl_available(&self) -> bool {
        Command::new(&self.kubectl)
            .arg("version")
            .arg("--client")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

async fn stop(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(launch_id): Path<Uuid>,
) -> Result<Json<LaunchStatus>, (StatusCode, Json<Value>)> {
    if !authorized(&headers, &state.token) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        ));
    }
    let name = pod_name(launch_id);
    if state.kubectl_available().await {
        let _ = Command::new(&state.kubectl)
            .args([
                "-n",
                &state.namespace,
                "delete",
                "pod",
                &name,
                "--ignore-not-found",
            ])
            .status()
            .await;
    }
    let _ = Command::new(&state.docker)
        .args(["rm", "-f", &name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;
    Ok(Json(LaunchStatus {
        status: "stopped".into(),
        runtime_id: Some(name),
        worker_id: None,
        error: None,
    }))
}

async fn logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(launch_id): Path<Uuid>,
) -> Result<Json<Logs>, (StatusCode, Json<Value>)> {
    if !authorized(&headers, &state.token) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        ));
    }
    let name = pod_name(launch_id);
    let docker_output = Command::new(&state.docker)
        .args(["logs", "--tail", "200", &name])
        .output()
        .await;
    let output = match docker_output {
        Ok(output) if output.status.success() => Ok(output),
        _ => {
            Command::new(&state.kubectl)
                .args(["-n", &state.namespace, "logs", &name, "--tail=200"])
                .output()
                .await
        }
    };
    let logs = match output {
        Ok(output) => {
            let mut text = String::from_utf8_lossy(&output.stdout).to_string();
            if !output.status.success() {
                text.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            text
        }
        Err(error) => format!("could not read logs: {error}"),
    };
    Ok(Json(Logs { logs }))
}
