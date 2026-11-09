# Runtime Adapter Contract

Rust source of truth: `crates/orbit-contracts/src/runtime.rs`.
Adapters: `crates/orbit-runtime/src/adapters/*`.

## Boundaries

- `orbit-api` never instantiates adapters. It persists intent and publishes events.
- `orbit-worker` resolves an adapter from the registry and executes it.
- Adapters may spawn subprocesses or containers, but receive no database handle.
- Adapters communicate progress only through the supplied event channel; they never
  write to `execution_events` directly.

## Trait

```rust
#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    fn kind(&self) -> RuntimeKind;
    fn capabilities(&self) -> RuntimeCapabilities;

    async fn validate_config(&self, cfg: &RuntimeConfig) -> Result<ValidationReport, RuntimeError>;
    async fn health(&self, cfg: &RuntimeConfig) -> Result<RuntimeHealth, RuntimeError>;

    // Long-running / instance-based runtimes. On-demand runtimes may return
    // `Unsupported`; the control plane then skips instance management.
    async fn create_instance(&self, req: CreateInstanceRequest) -> Result<RuntimeInstance, RuntimeError>;
    async fn start_instance(&self, inst: &RuntimeInstanceRef) -> Result<(), RuntimeError>;
    async fn stop_instance(&self, inst: &RuntimeInstanceRef) -> Result<(), RuntimeError>;
    async fn delete_instance(&self, inst: &RuntimeInstanceRef) -> Result<(), RuntimeError>;
    async fn instance_status(&self, inst: &RuntimeInstanceRef) -> Result<RuntimeInstanceStatus, RuntimeError>;

    async fn execute(&self, req: ExecutionRequest) -> Result<ExecutionOutcome, RuntimeError>;
    async fn resume(&self, req: ResumeRequest) -> Result<ExecutionOutcome, RuntimeError>;
}
```

`execute` is a long-running future. Cancellation and timeouts are delivered through
`ExecutionRequest::cancel: CancellationToken` and `ResourceLimits::timeout_seconds`;
adapters must terminate child processes/containers on cancellation.

## Capabilities

Advertised per adapter and surfaced to the UI (`GET /api/v1/runtimes`):

| Capability | Meaning |
|---|---|
| `supports_sessions` | runtime can hold conversation/session state |
| `supports_resume` | a prior session can be continued |
| `supports_streaming` | incremental log/tool events are emitted |
| `supports_cancel` | an in-flight execution can be cancelled |
| `supports_instances` | long-running instances are meaningful |
| `supports_tool_events` | tool/connector invocations are reported |
| `supports_usage_reporting` | token/CPU usage is reported |
| `execution_modes` | `[on_demand]`, `[long_running]` or both |
| `max_timeout_seconds` | upper bound on requested timeouts |
| `runtime_managed_providers` | runtime owns provider inventory/auth |

## Streaming events

`RuntimeEvent` variants: `Started { session_ref }`, `Log { level, message }`,
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
- `model`: either `RuntimeManaged { provider, model }` or `Endpoint { … }` with a
  resolved credential (never serialized into logs)
- `grants`: platform-approved connector capabilities (`connection_id`, `capability`,
  `effect`)
- `autonomy`: 1–4 with platform-enforced semantics
- `resources`: timeout, memory, CPU, max output tokens, max concurrent executions
- `runtime_config`: free-form, adapter-owned JSON (validated by `validate_config`)
- `workspace`: per-execution scratch directory contract
- `events`, `cancel`

## Error model

`RuntimeError { kind, message, retryable, details }` where `kind ∈ {invalid_config,
unauthorized, unavailable, unsupported, timeout, cancelled, execution_failed, protocol,
internal}`. Workers map `retryable` errors to bounded retries with exponential backoff;
non-retryable errors move the execution to `failed` immediately.

## Isolation requirements

Production adapters must run in a container/Job with: non-root user, read-only root
filesystem where possible, CPU/memory limits, `no_new_privileges`, a per-execution
writable workspace, allowlisted outbound network, secret injection via env or tmpfs
files, and forced teardown on timeout/cancel.

## Implementation status (Phase 0)

- `mock` — implemented, used for local development and contract tests.
- `opencode` — scaffolded adapter (`validate_config`, capability document, CLI
  preflight); real execution lands in Phase 1 against a tested OpenCode version.
- `claude_code`, `pi` — registered as capability descriptors only (Phase 2/3).
