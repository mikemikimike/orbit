# Provider & Connector Contracts

## Part A — Model providers

Source: `crates/orbit-contracts/src/provider.rs`. Providers are stored separately from
runtimes because a runtime may manage its own models, or consume an ORBIT-configured
endpoint.

### Kinds

| Kind | Use |
|---|---|
| `openai_compatible` | vLLM, LiteLLM, APISIX/AI gateway, OpenRouter-style, any `/v1` endpoint |
| `runtime_managed` | runtime owns auth/inventory (OpenCode Go, Claude Code subscription, Pi) |

OpenCode Go is **not** a runtime and not a generic provider: it is represented as a
`runtime_managed` provider whose credential is stored in `secret_ref` and handed to the
OpenCode adapter (Phase 1). It is never hardcoded in the UI or domain.

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
are decrypted only in memory and injected into runtime processes through env vars or
tmpfs files; they are never written to logs, execution events or the frontend.

## Part B — Connectors

Source: `crates/orbit-contracts/src/connector.rs` (design contract), implementation in
Phase 2.

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
    async fn validate_connection(&self, conn: &ConnectionContext) -> Result<(), ConnectorError>;
    async fn invoke(&self, ctx: ConnectorContext, call: CapabilityCall)
        -> Result<CapabilityResult, ConnectorError>;
    async fn verify_webhook(&self, headers: &HeaderMap, body: &[u8], secret: &SecretLease)
        -> Result<Option<ConnectorEvent>, ConnectorError>;
}
```

`ConnectorContext` contains only platform-approved grants; the connector SDK enforces
that `call.capability` is in the grant list before executing. MCP servers are one
transport option (`mcp` capability provider), not the only connector mechanism.

### Phase 0 scope

- Connector registry with seeded manifests (mock, generic-webhook, telegram, gitlab).
- Connections stored with encrypted credentials; create/list/delete only.
- Real invocation, webhooks, OAuth flows and MCP: Phase 2.
