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

## [Unreleased]

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
