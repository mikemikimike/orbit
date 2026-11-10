# Domain Model

## Aggregate relationships

```
organization 1─* user
organization 1─* agent 1─* agent_version
agent       1─* agent_instance *─1 harness
agent       1─* trigger
agent       1─* execution *─1 agent_version
execution   1─* execution_event
execution   1─* approval
organization 1─* connection *─1 connector
organization 1─* provider
organization 1─* policy
organization 1─* audit_log
```

## Entities

### Organization
Tenant boundary. `mode = personal | enterprise`. A personal organization is created
automatically for each user in self-hosted deployments. All queries are scoped by
`org_id`; enterprise RBAC builds on `users.role` and `policies`.

### User
`role ∈ {owner, admin, operator, viewer}`. Links to an external OIDC identity via
`(issuer, subject)`; never stores passwords.

### Agent (persistent definition)
| Field | Notes |
|---|---|
| `id`, `org_id`, `slug`, `name`, `description` | `slug` unique per org |
| `status` | `draft → active ⇄ paused → archived` |
| `current_version` | integer, points at an immutable `agent_versions` row |
| `labels` | JSONB for filtering |

A configuration change creates a new immutable version. Executions always reference the
exact `agent_version_id` they ran with.

### AgentVersion
`spec` JSONB is a validated `AgentSpec` (`apiVersion: orbit.io/v1`, `kind: Agent`).
`checksum` is the SHA-256 of canonical JSON; duplicate specs are rejected per agent.
`definition` retains the original YAML for export.

### AgentInstance
A deployed representation: `agent_version_id`, `harness_id`, `state`
(`pending → starting → running → stopping → stopped → failed`), `harness_instance_ref`,
`health`, harness-specific `config`. On-demand harnesses may not need instances; the
model supports both long-running and ephemeral execution.

### Harness (registration)
`kind ∈ {mock, opencode, claude_code, pi, generic}`. Carries a capability document
(`supports_sessions`, `supports_resume`, `supports_streaming`, `supports_cancel`,
`supports_long_running`, `max_timeout_seconds`, …). Health is reported by workers via
heartbeats; the control plane never probes harness internals directly.

### Provider
Model provider configuration **separate from harnesses**. `kind ∈
{openai_compatible, harness_managed}`. Secrets are `secret_ref` handles resolved by the
backend `SecretStore`; API responses redact them (`has_credential: true`).

### Connector / Connection / Capability / Policy
- **Connector**: integration definition (slug, version, capability catalog, auth kinds).
- **Connection**: authorized account for a connector in an org, with `credential_ref`,
  scopes and status (`active | expired | revoked | error`).
- **Capability**: `resource.action` (e.g. `issues.create`), classified by effect
  (`read | write | external_side_effect`).
- **Policy**: JSON document scoped to org/agent/connection; evaluated by the platform,
  not by the model.

### Trigger
`kind ∈ {manual, schedule, webhook, event, agent}`. Config validated per kind
(cron expression for `schedule`, HMAC secret reference for `webhook`, subject filter for
`event`). Triggers can be disabled without deleting them.

### Execution
Lifecycle:

```
queued → dispatched → running → succeeded
                    ├─→ failed
                    ├─→ timed_out
                    ├─→ cancelled
                    └─→ awaiting_approval → running | rejected
```

Fields: `trigger_source`, `input`, `output`, `error`, `session_ref`, `usage`,
`started_at`, `finished_at`, `idempotency_key` (unique). Terminal states are final.
Transitions are guarded in SQL (`UPDATE … WHERE status = ANY(allowed)`), so concurrent
workers cannot double-claim.

### ExecutionEvent
Append-only, per-execution monotonic `seq`: `kind ∈ {log, status, tool_call, output,
error, metric}`, `level`, `message`, `data`. Drives the SSE stream and the UI timeline.

### AuditLog
Append-only records of security-relevant actions (agent changes, secret changes,
connection grants, approvals, executions).

## Invariants

1. Every agent row has at least one immutable version.
2. Executions are never mutated in terminal states.
3. A capability invocation must be allowed by the agent's autonomy level **and** the
   connection's capability allowlist.
4. Secrets are write-only through the API.
5. Cross-organization references are rejected at the store layer.
