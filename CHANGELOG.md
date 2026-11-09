# Changelog

All notable changes to ORBIT are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/).

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

### Verified
- Helm lint/template for default and production values.
- Kubernetes deployment on a local cluster: all pods ready; web proxy → API → Postgres →
  worker → mock runtime → SSE smoke test passed.
- Private registry push of amd64 images to `192.168.1.201:5000`.

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
