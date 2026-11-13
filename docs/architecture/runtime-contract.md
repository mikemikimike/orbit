# Harness Adapter Contract

Rust source of truth: `crates/orbit-contracts/src/harness.rs`.
Adapters: `crates/orbit-harness/src/adapters/*`.

## Boundaries

- `orbit-api` never instantiates adapters. It persists intent and publishes events.
- `orbit-worker` resolves an adapter from the registry and executes it.
- Adapters may spawn subprocesses or containers, but receive no database handle.
- Adapters communicate progress only through the supplied event channel; they never
  write to `execution_events` directly.

## Trait

```rust
#[async_trait]
pub trait HarnessAdapter: Send + Sync {
    fn kind(&self) -> HarnessKind;
    fn capabilities(&self) -> HarnessCapabilities;

    async fn validate_config(&self, cfg: &HarnessConfig) -> Result<ValidationReport, HarnessError>;
    async fn health(&self, cfg: &HarnessConfig) -> Result<HarnessHealth, HarnessError>;

    // Long-running / instance-based harnesses. On-demand harnesses may return
    // `Unsupported`; the control plane then skips instance management.
    async fn create_instance(&self, req: CreateInstanceRequest) -> Result<HarnessInstance, HarnessError>;
    async fn start_instance(&self, inst: &HarnessInstanceRef) -> Result<(), HarnessError>;
    async fn stop_instance(&self, inst: &HarnessInstanceRef) -> Result<(), HarnessError>;
    async fn delete_instance(&self, inst: &HarnessInstanceRef) -> Result<(), HarnessError>;
    async fn instance_status(&self, inst: &HarnessInstanceRef) -> Result<HarnessInstanceStatus, HarnessError>;

    async fn execute(&self, req: ExecutionRequest) -> Result<ExecutionOutcome, HarnessError>;
    async fn resume(&self, req: ResumeRequest) -> Result<ExecutionOutcome, HarnessError>;
}
```

`execute` is a long-running future. Cancellation and timeouts are delivered through
`ExecutionRequest::cancel: CancellationToken` and `ResourceLimits::timeout_seconds`;
adapters must terminate child processes/containers on cancellation.

## Capabilities

Advertised per adapter and surfaced to the UI (`GET /api/v1/harnesses`):

| Capability | Meaning |
|---|---|
| `supports_sessions` | harness can hold conversation/session state |
| `supports_resume` | a prior session can be continued |
| `supports_streaming` | incremental log/tool events are emitted |
| `supports_cancel` | an in-flight execution can be cancelled |
| `supports_instances` | long-running instances are meaningful |
| `supports_tool_events` | tool/connector invocations are reported |
| `supports_usage_reporting` | token/CPU usage is reported |
| `execution_modes` | `[on_demand]`, `[long_running]` or both |
| `max_timeout_seconds` | upper bound on requested timeouts |
| `harness_managed_providers` | harness owns provider inventory/auth |

## Streaming events

`HarnessEvent` variants: `Started { session_ref }`, `Log { level, message }`,
`Status { phase }`, `ToolCall { name, input, output }`, `OutputDelta { text }`,
`Usage { tokens_in, tokens_out }`, `Error { message, retryable }`.

Events are forwarded by the worker to `execution_events` (with a monotonic `seq`) and to
`orbit.execution.updated` for subscribers. Adapters should emit events incrementally,
not only at completion.

## Inputs

`ExecutionRequest` carries:

- `execution_id`, `agent_id`, `agent_version`, `agent_name`
- trusted `instructions` (system prompt) — kept separate from untrusted user/event content
- `input`: task text, structured payload, and provenance (`source`, `trusted: false`)
- `model`: either `HarnessManaged { provider, model }` or `Endpoint { … }` with a
  resolved credential (never serialized into logs)
- `grants`: platform-approved connector capabilities (`connection_id`, `capability`,
  `effect`)
- `autonomy`: 1–4 with platform-enforced semantics
- `resources`: timeout, memory, CPU, max output tokens, max concurrent executions
- `harness_config`: free-form, adapter-owned JSON (validated by `validate_config`)
- `workspace`: per-execution scratch directory contract
- `events`, `cancel`

## Error model

`HarnessError { kind, message, retryable, details }` where `kind ∈ {invalid_config,
unauthorized, unavailable, unsupported, timeout, cancelled, execution_failed, protocol,
internal}`. Workers map `retryable` errors to bounded retries with exponential backoff;
non-retryable errors move the execution to `failed` immediately.

## Placement and worker targets

Agents may set `spec.harness.placement`:

| Value | Claimed by |
|---|---|
| `auto` | any worker supporting the harness |
| `local` | a worker running natively on the host (`ORBIT_WORKER_TARGET=local`) |
| `docker` | a Docker/Compose worker or the local container dispatcher (`docker`) |
| `kubernetes` | workers deployed by the Helm chart (`kubernetes`) |

Placement is a routing constraint, not an isolation guarantee. Isolation depends on the
worker's execution driver (process vs per-execution container/Job). Workers report target
and harnesses in `worker_heartbeats`; `GET /api/v1/workers` exposes them.

## Isolation requirements

Production adapters must run in a container/Job with: non-root user, read-only root
filesystem where possible, CPU/memory limits, `no_new_privileges`, a per-execution
writable workspace, allowlisted outbound network, secret injection via env or tmpfs
files, and forced teardown on timeout/cancel.

## Implementation status

- `mock` — implemented, used for local development and contract tests.
- `opencode` — **available**, tested against OpenCode **v2.3.0**:
  - `opencode run --standalone --format json` in a per-execution workspace
  - trusted instructions written to `AGENTS.md`; untrusted input passed in the prompt
  - NDJSON text/tool/usage/error events mapped to ORBIT harness events
  - session IDs captured; `--session` used for resume; cancellation kills the process group
  - `PWD` and the working directory are pinned to the per-execution workspace
  - harness-managed providers via `opencode auth export` imported from a mounted Secret;
    ORBIT never passes raw provider keys in CLI arguments
  - **platform providers supported**: ORBIT-resolved OpenAI-compatible endpoints (vLLM,
    Ollama, gateways) are written into `opencode.json` with their base URL, headers and
    leased credential (`supports_platform_providers=true`)
  - GitHub tools (read and approval-gated write) verified end-to-end through the MCP
    tool broker
  - contract + real-harness test: `crates/orbit-harness/tests/opencode_e2e.rs`
    (`ORBIT_OPENCODE_E2E=1`, ignored by default to avoid model spend)
- `pi` — **available**, tested against Pi **1.5.2**:
  - `pi --print --mode json`, session capture/resume, JSON event mapping,
    cancellation/timeout and trusted `--append-system-prompt`
  - platform-enforced read-only tool allowlist (`read,grep,find,ls`) at autonomy L1/L2
  - **platform providers supported**: endpoint and leased credential injected as
    `OPENAI_API_KEY` / `OPENAI_BASE_URL`, with `--provider openai` when a platform model
    is selected (`supports_platform_providers=true`)
  - writes the execution-scoped `.pi/mcp.json` broker bridge and accepts `--skill` paths
  - ships as its own worker image (`deploy/docker/worker-pi.Dockerfile`)
- `claude_code` — registered as a capability descriptor only, fails closed (planned).
