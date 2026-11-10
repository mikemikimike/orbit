//! MCP stdio bridge.
//!
//! Agent harnesses (OpenCode) spawn this binary as a local MCP server. It forwards
//! `tools/list` and `tools/call` to the ORBIT tool broker over loopback, authenticated
//! with an execution-scoped token. The harness never receives provider credentials; all
//! policy, approval and connector logic lives in the broker.

use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROTOCOL_VERSION: &str = "2024-11-05";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let broker_url = std::env::var("ORBIT_BROKER_URL")
        .map_err(|_| anyhow::anyhow!("ORBIT_BROKER_URL is required"))?;
    let token = std::env::var("ORBIT_BROKER_TOKEN")
        .map_err(|_| anyhow::anyhow!("ORBIT_BROKER_TOKEN is required"))?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60 * 30))
        .build()?;

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("orbit-mcp-broker: invalid JSON-RPC: {error}");
                continue;
            }
        };
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(Value::Null);

        // Notifications have no id and must not produce a response.
        if id.is_none() {
            continue;
        }
        let id = id.unwrap();

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "orbit-broker", "version": env!("CARGO_PKG_VERSION") }
                }
            }),
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            "tools/list" => match broker_get(&client, &broker_url, &token, "/tools").await {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err(error) => tool_bridge_error(id, &error),
            },
            "tools/call" => {
                match broker_post(&client, &broker_url, &token, "/tools/call", &params).await {
                    Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                    Err(error) => tool_bridge_error(id, &error),
                }
            }
            other => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {other}") }
            }),
        };

        let mut encoded = serde_json::to_vec(&response)?;
        encoded.push(b'\n');
        stdout.write_all(&encoded).await?;
        stdout.flush().await?;
    }
    Ok(())
}

fn tool_bridge_error(id: Value, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{ "type": "text", "text": format!("ORBIT tool broker error: {message}") }],
            "isError": true
        }
    })
}

async fn broker_get(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    path: &str,
) -> Result<Value, String> {
    let response = client
        .get(format!("{}{}", base_url.trim_end_matches('/'), path))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("broker request failed: {e}"))?;
    parse_response(response).await
}

async fn broker_post(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    path: &str,
    body: &Value,
) -> Result<Value, String> {
    let response = client
        .post(format!("{}{}", base_url.trim_end_matches('/'), path))
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("broker request failed: {e}"))?;
    parse_response(response).await
}

async fn parse_response(response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| format!("broker returned invalid JSON: {e}"))?;
    if !status.is_success() {
        let message = body
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("broker request failed");
        return Err(format!("{status}: {message}"));
    }
    Ok(body)
}
