# API Design (`/api/v1`)

OpenAPI document: `GET /api/v1/openapi.json`; interactive docs: `GET /docs`.
The TypeScript client in `packages/api-client` is generated from this document.

## Conventions

- JSON only (`application/json`), UTC RFC 3339 timestamps, UUIDv7 identifiers.
- Authentication: `Authorization: Bearer <access_token | api_key>`. Native clients use
  short-lived access tokens + refresh tokens; automation uses `orbit_…` API keys.
- Every response carries `X-Request-Id`; errors include it.
- Idempotent POSTs accept `Idempotency-Key` (executions).
- Keyset pagination: `?limit=25&cursor=<opaque>` → `{ "items": [...], "next_cursor": null }`.
- Filtering: `?status=running&agent_id=...&q=...` where documented.
- Optimistic concurrency: `PATCH` may include `If-Match: "<version>"`.

### Error envelope

```json
{
  "error": {
    "code": "validation_error",
    "message": "agent spec is invalid",
    "details": [{ "path": "spec.resources.timeoutSeconds", "message": "must be 1..=86400" }],
    "request_id": "01J…"
  }
}
```

Codes: `bad_request`, `validation_error`, `unauthorized`, `forbidden`, `not_found`,
`conflict`, `rate_limited`, `payload_too_large`, `unsupported`, `internal`.

## Resources

| Method & path | Purpose |
|---|---|
| `GET /healthz`, `GET /readyz` | liveness / dependency readiness |
| `GET /metrics` | Prometheus metrics |
| `GET /api/v1/meta` | platform version, auth mode, capabilities |
| `GET /api/v1/me` | current user, organization, role, permissions |
| `GET/POST /api/v1/agents` | list (filter/search) / create agent + version 1 |
| `GET/PATCH/DELETE /api/v1/agents/{id}` | read / new version or metadata / archive |
| `POST /api/v1/agents/{id}/activate` | draft/paused → active |
| `POST /api/v1/agents/{id}/pause` | active → paused |
| `GET /api/v1/agents/{id}/versions` | immutable version history |
| `GET /api/v1/agents/{id}/export` | YAML export (`orbit.io/v1`) |
| `POST /api/v1/agents/{id}/executions` | manual execution (202) |
| `GET /api/v1/agents/{id}/executions` | executions for agent |
| `GET /api/v1/executions` | all executions (status, agent, cursor) |
| `GET /api/v1/executions/{id}` | execution detail with output/error/usage |
| `GET /api/v1/executions/{id}/events` | stored events (paginated, `after_seq`) |
| `GET /api/v1/executions/{id}/events/stream` | SSE stream (replay + tail) |
| `POST /api/v1/executions/{id}/cancel` | request cancellation |
| `GET /api/v1/runtimes` | runtime registry + capabilities + health |
| `GET /api/v1/runtimes/{kind}` | runtime detail, adapter status |
| `GET/POST /api/v1/providers` | list / create model provider |
| `PATCH/DELETE /api/v1/providers/{id}` | update / delete (redacted secrets) |
| `POST /api/v1/providers/{id}/test` | connectivity probe (OpenAI-compatible) |
| `GET /api/v1/connectors` | connector registry/manifests |
| `GET/POST /api/v1/connections` | list / create connection |
| `DELETE /api/v1/connections/{id}` | revoke connection |
| `GET/POST /api/v1/triggers` | list / create trigger |
| `PATCH/DELETE /api/v1/triggers/{id}` | update / delete |
| `POST /api/v1/webhooks/{trigger_id}` | inbound webhook (HMAC verified) |
| `GET /api/v1/organizations/current` | current organization |
| `GET /api/v1/users` | organization members |
| `GET /api/v1/dashboard/summary` | dashboard counters + recent activity |
| `GET /api/v1/audit-logs` | audit trail (admin) |

Phase 0 implements the resources marked above except webhooks, audit listing filters
and provider probing beyond a basic `/models` check (documented limits).

## SSE stream

```
event: execution.event
id: 42
data: {"seq":42,"kind":"log","level":"info","message":"…","ts":"…"}
```

The endpoint sets `Last-Event-ID` support, sends a `snapshot` event first (execution
status), then replays stored events with `seq > last_event_id` and polls for new rows.
Terminal executions close the stream after a final `done` event.

## Auth model

- Dev mode (`ORBIT_AUTH_MODE=dev`): a seeded developer user/organization; any
  authenticated-ish token is accepted only when `ORBIT_DEV_AUTH_ALLOW_ANONYMOUS=true`.
- Production (`ORBIT_AUTH_MODE=oidc`): Authorization Code + PKCE handled by the web app
  against the configured OIDC provider; ORBIT validates bearer JWTs against JWKS and
  maps `(iss, sub)` to a user. API keys (`orbit_…`) are hashed (SHA-256) in
  `api_tokens` and scoped.
- All authorization checks live in `orbit-security`/handlers, never in prompts.
