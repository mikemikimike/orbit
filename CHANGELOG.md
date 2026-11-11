# Changelog

All notable changes to ORBIT are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.4] — Secure-context fix for plain HTTP deployments

### Added
- Architecture drawings: `docs/architecture/orbit-overview.svg` / `.png` and detailed
  Mermaid diagrams for context, execution sequence, broker policy path, domain model and
  deployment topology (`docs/architecture/diagram.md`).

### Fixed
- Agent pages crashed with `crypto.randomUUID is not a function` when ORBIT is served over
  plain HTTP on a LAN hostname (Web Crypto UUIDs and the async clipboard API require a
  secure context). Added secure-context-safe `randomId()` and `copyText()` helpers with
  fallbacks, so the UI works over HTTP until TLS is configured. TLS is still recommended
  for PWA/service-worker, clipboard and browser security.

## [0.1.3] — Real OpenCode runtime + X read connector

### Added
- **OpenCode runtime execution** (`opencode run --standalone --format json`): NDJSON
  event mapping (text, tool calls, usage, errors), session capture/resume, cancellation
  via process-group termination, timeouts, per-execution workspace, and trusted
  instructions written to `AGENTS.md`. Tested against OpenCode v2.2.5 with a real
  integration test (`ORBIT_OPENCODE_E2E=1`).
- Separate `orbit-worker-opencode` image and Helm Deployment; workers now claim only the
  runtimes in `ORBIT_WORKER_RUNTIMES` (`mock` for the base worker, `opencode` for this
  one). OpenCode credentials are imported from an `opencode auth export` Secret by an
  init container.
- **X (Twitter) read connector**: `posts.search` hydration before execution using the
  encrypted connection bearer token; posts are injected as untrusted `externalContext`
  and a query can come from payload/connection config. API failures fail the execution
  with an actionable message.
- Agent Studio connection picker per connector binding and an execution payload/query
  field in the Run dialog.

### Fixed
- Agent detail page crashed when an agent had connector bindings because the API
  serialized `connectionId: null` and the shared Zod schema rejected null. The API now
  omits null IDs, the schema accepts nullish values, and the page renders a validation
  fallback instead of crashing.
- OpenCode worker needs writable XDG data/state/config directories; the chart mounts
  writable emptyDirs and imports auth before start.
- Worker startup message no longer claims OpenCode is only scaffolded.

### Verified on microk8s (`http://orbit.mdk.local`)
- Real OpenCode execution in-cluster: `succeeded`, session captured, 6.1k input / 10
  output tokens, streamed events.
- Cancellation of a long OpenCode execution: `cancelled`, process terminated.
- X connector call returns a clear 401 with an invalid token (real API request path).
- Existing mock-runtime smoke test and SSE through ingress continue to pass.

## [0.5.1] — worker launcher

### Added
- Optional `orb-launcher` service with Docker and Kubernetes Pod drivers, protected by
  an internal launcher token. The API never receives Docker socket or workload RBAC.
- Durable `worker_launches` records with status, runtime id, worker id and errors.
- Workers UI: Launch, Stop, status and last-200-lines logs.
- Helm launcher Deployment, namespaced Pod RBAC and launcher token plumbing; Compose
  launcher with Docker socket.
- API endpoints for launching profiles and listing/stopping/inspecting launches.

### Changed
- Worker launches use profile-scoped, per-launch tokens that are revoked on stop.
- Launcher is opt-in; without it the API returns a clear `launcher_unavailable` error.

## [0.5.0] — worker tokens and token-only bootstrap

### Added
- Scoped worker tokens (`orbit_wk_`), stored as SHA-256 hashes with org/profile/harness
  scoping, revocation and last-used tracking.
- Worker API: session, heartbeat, claim, execution context, running, events, completion,
  cancellation, approvals, audit and connector credential leases.
- Worker profiles issue/rotate a token when generating bootstrap output.
- Worker token administration endpoints: list, create, revoke.
- Scheduled triggers and orphaned-execution reaping moved into `orbit-api`.

### Changed
- Workers no longer receive `ORBIT_DATABASE_URL`, `ORBIT_NATS_URL` or
  `ORBIT_MASTER_KEY`. They use `ORBIT_API_URL` + `ORBIT_WORKER_TOKEN`.
- Per-execution containers inherit the worker token instead of database credentials.
- Helm and Compose now take worker tokens (`worker.token`, `opencodeWorker.token`,
  `ORBIT_WORKER_TOKEN`).
- Model resolution, connector credentials, skills and approvals are served by the
  control plane.
- Worker bootstrap manifests contain the token and no database/master-key placeholders.

### Removed
- Worker-side scheduler, reaper and direct PostgreSQL/NATS event publishing.

### Known limitations
- Token rotation in Bootstrap invalidates existing workers using the old token.
- Per-execution Kubernetes Jobs remain pending.

## [0.4.0] — Harness rename

### Changed (breaking)
- Renamed the agent-framework concept from **runtime** to **harness** across domain,
  database, API, worker, profiles, UI and docs.
- DB: `runtimes` → `harnesses`, `runtime_kind` → `harness_kind`, `runtime_id` →
  `harness_id`, `worker_heartbeats.runtimes` → `harnesses`; agent specs
  `spec.runtime` → `spec.harness`; provider kind `runtime_managed` →
  `harness_managed`.
- API: `/api/v1/harnesses` (was `/runtimes`); `runtime`/`runtimeKind` →
  `harness`/`harnessKind`; permission `harnesses:read`; regenerated TypeScript client.
- Env: `ORBIT_WORKER_HARNESSES`, `ORBIT_EXECUTION_IMAGE_<HARNESS>`; worker profile field
  `harnessKind`.
- UI: Harnesses and Providers navigation; Agent Studio "Harness & model".

### Added
- ADR-0008 defining harness/worker/sandbox/provider/model terminology and the
  worker-token + launcher design.

### Known limitations
- Worker token/API mode and the UI launcher are the next milestone; workers still use
  the database in 0.4.0.
- Worker bootstrap output still uses DB/NATS/master key until that API exists.

## [0.3.0] — skills, worker profiles and Pi runtime

### Added
- **Skills library**: org-scoped `SKILL.md` packages with supporting files, versioning,
  CRUD API/UI and audit. Attach skills to agents in Agent Studio.
- Worker materializes attached + profile-level skills into `.pi/skills/<slug>` and
  `.opencode/skills/<slug>` before execution.
- **Worker profiles**: pin runtime, image/tag, target, execution driver, one or more
  agents, shared skills, provider/model metadata and an external auth secret reference.
  Workers load `ORBIT_WORKER_PROFILE_ID`, claim only their agent allowlist and materialize
  profile skills.
- Bootstrap generator in the UI/API (`/worker-profiles/{id}/bootstrap`) for Docker,
  Compose and Kubernetes output, including runtime auth mount guidance.
- **Pi runtime adapter** tested against Pi 1.5.0: headless JSON, sessions, read-only tools
  at L1/L2, cancellation/timeout and `.pi/mcp.json` broker bridge generation.
- Pi worker image (`worker-pi.Dockerfile`, Node 22 + Pi 1.5.0) and `make worker-pi` host
  runner.
- `GET /api/v1/workers` and Active workers UI; queued-execution diagnostics.

### Changed
- Version 0.3.0.
- Skills are validated (slug, required SKILL.md, safe relative files) before persistence.

### Known limitations
- Pi worker Helm Deployment and Pi ↔ broker MCP parity are not yet verified end-to-end.
- Worker profile output is a starting manifest; it does not yet launch containers from
  the control plane.
- OpenCode skill discovery depends on its project skill directory; verify per runtime
  version. Agent presets/runtime-native subagents remain runtime-config JSON.

## [0.2.1] — explicit placement and queued-execution diagnostics

### Added
- `runtime.placement` on agents (`auto`, `local`, `docker`, `kubernetes`), editable in
  Agent Studio and shown on the agent/execution detail. Workers advertise a target in
  their heartbeat; the claim query only takes `auto` or matching-target executions.
- `GET /api/v1/workers` and an “Active workers” section on Runtimes showing runtimes,
  target, hostname and liveness.
- Queued executions now get a timeline event and an in-UI diagnostic that checks for a
  matching live worker, with instructions for starting one.
- Worker target env defaults: host native `local`, Compose/Docker dispatcher `docker`,
  Kubernetes workers `kubernetes`.

### Fixed
- Reaper grace now uses claim time (`started_at`) instead of creation time, so retrying an
  older queued execution cannot be reaped before its worker writes the first heartbeat.
- `/api/v1/workers` route was missing from the router.

## [0.2.0] — approvals, worker recovery, per-execution containers

### Added
- **Approvals UI**: pending queue with agent/execution context, argument preview,
  approve/reject with comment, recent decisions, navigation badge and dashboard banner.
  Backend decisions are audited; pending approvals expire when their execution reaps.
- **Worker lease/reaper**: `worker_heartbeats` table; workers heartbeat every 10s; a
  periodic reaper fails executions whose worker heartbeat is stale (15s grace + threshold)
  and expires their pending approvals. Fixes stuck `running` rows after crashes/restarts.
- **Per-execution Docker containers** (host dispatcher): `ORBIT_EXECUTION_DRIVER=docker`
  starts a short-lived container per execution running `orbit-worker` in
  `ORBIT_SINGLE_EXECUTION_ID` mode. The container has its own tool broker/MCP bridge,
  workspace and heartbeat; the parent holds a concurrency permit and waits for exit.
  Verified locally with mock and OpenCode + GitHub read in a spawned container.
- `make worker-docker`, `scripts/dev-worker-docker.sh`, `make compose-build-opencode` and
  host-architecture OpenCode image builds (`OPENCODE_ARCH`).
- Real GitHub read verified end-to-end: OpenCode → MCP → broker → GitHub `issues.list`
  → summarized output, including inside a per-execution Docker container.

### Changed
- Execution timeline hides reasoning, step and usage noise by default; generic internal
  tools are debug-only. “Show verbose” reveals the raw stream.
- OpenCode `PWD` is pinned to the per-execution workspace; verified `pwd` isolation.
- OpenCode stderr diagnostics gated behind `debugLogs` (off by default).

### Known limitations
- Kubernetes still runs the worker in process mode; Job-per-execution dispatch is the
  next isolation milestone. Docker dispatch requires Docker socket/CLI access and is for
  single-user/homelab deployments.
- GitHub write/approval E2E against a real repository still needs a PAT; the broker and
  approvals UI are implemented, and the approval UI was verified with a real pending
  approval record.
- Orphaned executions are failed, not automatically retried (idempotent retries next).

## [Unreleased]

### Added
- **Pi runtime adapter** (tested against Pi 1.5.0): headless JSON mode, session capture,
  cancellation/timeout, trusted system append, `.pi/mcp.json` broker bridge generation,
  and a platform-enforced read-only tool allowlist at autonomy L1/L2.
- `make worker-pi` / `scripts/dev-worker-pi.sh` for local host execution.
- Pi config accepts `skills` paths and passes them via `--skill`.
- Note: Pi worker container image/Helm deployment and full MCP tool parity are not yet
  verified; the UI skill/agent registry remains pending.

### Added
- Real GitHub read flow verified end-to-end locally: OpenCode → MCP bridge → tool broker
  → GitHub REST (`issues.list` on a public repo) → summarized output, no mock runtime.
- Execution timeline noise reduction: reasoning/step/usage events are not shown by
  default; generic internal tool calls are debug-only; GitHub/approval/error tool calls
  stay visible; “show verbose” reveals the raw stream.
- `debugLogs` runtime setting for OpenCode stderr diagnostics (off by default).

### Fixed
- OpenCode ran in the worker's working directory instead of the per-execution workspace
  on some hosts (inherited `PWD`). The adapter now sets `PWD` to the workspace; verified
  with `pwd` and process cwd inspection.
- `step_start`/usage events no longer spam execution timelines (usage is aggregated).

### Known limitation
- Executions still run inside a long-lived worker container/host process, not a fresh
  container per execution. Stale `running` rows after a worker crash need a lease/reaper
  (next reliability milestone).

### Added
- Documented and verified full local Docker Compose stack (db, NATS, API, mock worker,
  web) with health checks, same-origin API proxy, demo seeding and fast restart workflow.
- `make compose-logs`, `make compose-reset`, `make worker-opencode` and
  `scripts/dev-worker-opencode.sh` for running a host OpenCode worker against Compose.
- Execution-scoped tool broker foundation: policy/autonomy checks, approvals store + API,
  GitHub connector (read + approval-gated writes), MCP stdio bridge and worker wiring
  (approvals UI and deployment verification still pending).

### Added
- Responsive web experience: dashboard, agents list, Agent Studio, agent detail,
  executions list, live execution timeline (SSE via fetch + auth header), runtimes,
  models/providers, connections and settings.
- Docker images (multi-stage, non-root, `linux/amd64`) for API, worker and web, plus a
  full Compose stack and an infrastructure-only Compose file.
- Helm chart with bundled or external PostgreSQL/NATS, API/worker/web deployments,
  Services, SSE-friendly nginx ingress at `orbit.mdk.local`, probes, resources,
  ServiceAccount, secret references, optional Vault env wiring and a production example.
- Deployment scripts: image build/push to the private registry, Helm deploy and an
  end-to-end Kubernetes smoke test.
- Web container API proxy (same-origin `/api/*`) with runtime `ORBIT_API_INTERNAL_URL`.

### Changed
- Idempotency keys for executions are scoped per organization (migration `0002`).
- Ingress host corrected to `orbit.mdk.local`.
- `/api` is routed through the web container's API proxy so dev auth tokens are injected
  server-side and OIDC/API bearer headers pass through transparently. `/readyz` is
  routed to the API alongside `/healthz`.
- Worker waits up to 90s for the API to bootstrap/migrate on fresh installs instead of
  crash-looping (Helm install ordering).

### Verified
- Helm lint/template for default and production values.
- Kubernetes deployment on a local cluster: all pods ready; web proxy → API → Postgres →
  worker → mock runtime → SSE smoke test passed.
- Private registry push of amd64 images to `192.168.1.201:5000`.
- Live microk8s deployment at `http://orbit.mdk.local` (ingress address
  `192.168.1.200`): ingress → web proxy → API → PostgreSQL → worker → mock runtime,
  smoke test passed, SSE verified through the ingress.

## [0.1.0] — Phase 0 foundation

### Added
- **Monorepo** with a Rust workspace (`crates/*`, `services/*`), pnpm workspaces
  (`apps/*`, `packages/*`) and shared OpenAPI-generated TypeScript client.
- **Domain model** (`orbit-domain`): versioned `orbit.io/v1` agent definitions with
  semantic validation, runtime-agnostic specs, autonomy levels 1–4, capability
  evaluation, execution state machine, keyset pagination cursors.
- **Contracts** (`orbit-contracts`): `RuntimeAdapter` trait with capabilities, lifecycle,
  execute/resume, streaming events, cancellation and typed errors; provider contracts for
  OpenAI-compatible and runtime-managed providers with explicit compatibility flags;
  connector manifest/capability/effect model; event envelope and NATS subject filter.
- **Persistence** (`orbit-store`): 18-table PostgreSQL schema with migrations covering
  organizations, users, API tokens, agents, immutable agent versions, instances,
  runtimes, providers, connectors, connections, triggers, executions, execution events,
  approvals, policies, audit logs, processed events and encrypted secrets. Keyset
  pagination, org scoping, atomic execution claiming, terminal-state guards.
- **Security** (`orbit-security`): OIDC JWT validation with JWKS caching, hashed API
  tokens, role/permission matrix, AES-256-GCM secret store with reference-bound AAD.
- **Event transport** (`orbit-events`): `EventBus` abstraction with in-memory and NATS
  JetStream drivers; durable PostgreSQL queue as the source of truth with polling
  failover.
- **Runtime registry** (`orbit-runtime`): mock runtime (fully working, streaming,
  cancellation, timeouts) plus OpenCode adapter scaffold (capabilities, config
  validation, CLI preflight) and registered descriptors for Claude Code, Pi and generic.
  Shared contract test harness.
- **Control plane API** (`orbit-api` + `orbit-api` binary): health/readiness/metrics,
  OpenAPI 3.1 document + Swagger UI, agents CRUD/versioning/export/activate/pause,
  manual executions with idempotency, concurrency and rate limits, execution events and
  SSE streaming with replay, runtimes, providers (with write-only credentials and
  `/models` probing), connectors, connections, triggers, dashboard, audit log, API token
  management and org/user endpoints. Dev auth mode and OIDC bearer validation.
- **Execution plane worker** (`orbit-worker`): durable queue polling, concurrency
  limiting, runtime dispatch, incremental event persistence, cancellation watcher,
  timeout enforcement, scheduled trigger firing and rescheduling.
- **Frontend foundation**: Next.js App Router + Tailwind + shadcn-style components,
  TanStack Query, Zod schemas, typed OpenAPI client, mobile-first shell and PWA assets.
- **Deployment**: Dockerfiles (multi-stage, non-root) and Helm chart targeting
  `orbit.mdk.local` ingress; Compose stack for local development.
- **Documentation**: architecture overview, domain model, runtime/provider/connector
  contracts, events, API design, ADRs 0001–0007, local development, testing and
  deployment guides.

### Known limitations
- The OpenCode adapter does **not** execute tasks yet; it validates configuration and
  performs a real CLI preflight. Do not advertise it as working until Phase 1
  integration tests pass against a pinned OpenCode version.
- Claude Code, Pi and generic runtimes are descriptor-only and fail closed.
- Connector capability invocation, OAuth flows, webhook verification and approvals are
  Phase 2.
- Runtime isolation (containers/Jobs per execution) is not implemented yet; the mock
  runtime runs in the worker process. The production execution path must not run
  untrusted tasks in the worker process.
- OIDC login (Authorization Code + PKCE) is validated server-side, but the web app does
  not yet implement the browser login flow; dev auth is the supported local mode.
- No RBAC management UI beyond role changes; enterprise features are Phase 4.
