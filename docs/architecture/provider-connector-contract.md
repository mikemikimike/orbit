# Provider & Connector Contracts

## Part A — Model providers

Source: `crates/orbit-contracts/src/provider.rs`. Providers are stored separately from
harnesses because a harness may manage its own models, or consume an ORBIT-configured
endpoint.

### Kinds

| Kind | Use |
|---|---|
| `openai_compatible` | vLLM, LiteLLM, APISIX/AI gateway, OpenRouter-style, any `/v1` endpoint |
| `harness_managed` | harness owns auth/inventory (OpenCode Go, Claude Code subscription, Pi) |

OpenCode Go is **not** a harness and not a generic provider: it is represented as a
`harness_managed` provider whose credential is stored in `secret_ref` and handed to the
OpenCode adapter. Provider names and model inventories are never hardcoded in the UI or
domain; both are discovered from the provider registry.

### `openai_compatible` configuration

```jsonc
{
  "base_url": "https://gateway.example.com/v1",
  "model": "qwen3-coder",
  "secret_ref": "provider/9f0c…",          // never returned by the API
  "headers": { "X-Tenant": "orbit" },       // optional, non-secret custom headers
  "request_params": { "temperature": 0.2 }, // optional, adapter passthrough
  "context_window": 131072,
  "max_output_tokens": 16384,
  "compat": {
    "chat_completions": true,
    "responses_api": false,
    "tools": true,
    "streaming": true,
    "json_mode": false,
    "vision": false,
    "models_endpoint": true
  }
}
```

Capabilities are **explicit**: adapters must not assume every OpenAI-compatible endpoint
supports tools, streaming, JSON mode or `/models`. Unsupported features degrade
predictably and are visible in the UI.

### Resolution

The worker resolves a provider into `ResolvedModel` for the adapter, which includes the
endpoint, model id, custom headers, params and a short-lived `SecretLease`. Credentials
are decrypted only in memory and injected into harness processes through env vars or
tmpfs files; they are never written to logs, execution events or the frontend.

## Part B — Connectors

Source: `crates/orbit-contracts/src/connector.rs` (trait + DTOs), manifests in
`crates/orbit-api/src/connectors.rs`, execution-plane clients in
`services/worker/src/{github,gitlab}.rs`.

### Concepts

- **Connector**: versioned integration manifest with auth methods and capability catalog.
- **Connection**: authorized account in an organization (`credential_ref`, scopes,
  status, expiry).
- **Capability**: `resource.action` with an effect:
  - `read` — safe to call at autonomy level 1+ (Observe)
  - `write` — mutating, requires level 3 (prepare/approval) or 4 with allowlist
  - `external_side_effect` — sends messages, creates MRs, spends money; always
    auditable and approval-gated below level 4

### Manifest

```jsonc
{
  "slug": "gitlab",
  "name": "GitLab",
  "version": "0.1.0",
  "auth": ["oauth2", "api_token"],
  "webhooks": true,
  "capabilities": [
    { "id": "issues.list", "effect": "read" },
    { "id": "issues.create", "effect": "write" },
    { "id": "pipelines.status", "effect": "read" }
  ]
}
```

### Invocation contract

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    fn manifest(&self) -> &ConnectorManifest;
    async fn validate_connection(&self, context: &ConnectorContext) -> Result<(), ConnectorError>;
    async fn invoke(&self, context: ConnectorContext, call: CapabilityCall)
        -> Result<CapabilityResult, ConnectorError>;
    async fn verify_webhook(&self, headers: &http::HeaderMap, body: &[u8],
        secret: Option<&SecretLease>) -> Result<Option<ConnectorEvent>, ConnectorError>;
}
```

**Current implementation note:** the built-in GitHub and GitLab connectors are invoked by
the worker's policy-enforcing tool broker (`services/worker/src/broker.rs`) through the
`github.rs` / `gitlab.rs` clients rather than as `Connector` trait objects. The trait
remains the target contract for third-party connectors. X read hydration is a separate
pre-execution step.

`ConnectorContext` contains only platform-approved grants; the connector SDK enforces
that `call.capability` is in the grant list before executing. MCP servers are one
transport option (`mcp` capability provider), not the only connector mechanism.

### Implemented scope

- **Manifests** seeded for `mock`, `telegram`, `github`, `gitlab`, `x`, `webhook`.
- **Connections** stored with encrypted credentials; create / list / update / delete.
  The API never returns the credential (`hasCredential` only).
- **GitHub** through the tool broker: `issues.list`, `issues.get`, `issues.comment`,
  `issues.create`, `issues.update`, `pulls.list`, `pulls.get`, `pulls.create`,
  `checks.list` (reads implemented, writes approval-gated and needing live E2E).
- **GitLab** through the tool broker: `projects.list`, `group_issues.list`,
  `boards.list`, `issues.*`, `merge_requests.*`, `pipelines.status`. API-token auth.
- **X read hydration**: before an execution, the worker resolves `x` bindings with the
  `posts.search` capability, loads the connection's bearer token, calls
  `GET /2/tweets/search/recent`, and injects up to 10 posts into the harness input as
  `externalContext` with `provenance.trusted=false`. The query comes from
  `payload.query`, then `connection.config.query`, then the task text.
- **Repository tools** (`repo.checkout`, `repo.commit`, `repo.push`) run in the worker
  over a per-execution workspace; git credentials are injected through `GIT_ASKPASS` in
  the child process only.
- **Capability enforcement**: the broker evaluates `evaluate_capability` (autonomy ×
  effect × grant) before every call; writes below L4 create approvals.

Not yet implemented: X posting, Telegram `messages.send` invocation, OAuth flows, webhook
verification, and a third-party connector SDK.
