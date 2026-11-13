# ORBIT — Open Agent Platform

ORBIT is a self-hosted **control plane** for creating, configuring, connecting, deploying
and governing persistent AI agents. It is deliberately *not* another agent framework:
agent reasoning runs in dedicated harness processes behind a stable adapter contract,
while ORBIT owns identity, configuration, orchestration, policy and history.

```
Browser / future mobile app
        │  REST /api/v1 + SSE (OpenAPI contract)
┌───────▼────────────────────────────────────────────────┐
│ Control plane: orbit-api (Axum)                        │
│   agents · executions · providers · connectors · policy │
│   PostgreSQL (durable state) · NATS JetStream (events)  │
└───────▲────────────────────────────────────────────────┘
        │ HarnessAdapter contract
┌───────▼────────────────────────────────────────────────┐
│ Execution plane: orbit-worker                          │
│   mock (working) · opencode (available, v2.3.0)        │
│   pi (available, v1.5.2) · claude_code / generic (planned)│
└─────────────────────────────────────────────────────────┘
```

## Architecture

![ORBIT architecture overview](docs/architecture/orbit-overview.svg)

Detailed diagrams (system context, execution sequence, broker policy path, domain model,
deployment topology): [`docs/architecture/diagram.md`](docs/architecture/diagram.md).

## Status

Phase 0 foundation is implemented and validated:

- Rust workspace: domain, contracts, store, events, security, harness adapters, HTTP API,
  worker.
- PostgreSQL schema with migrations (18 tables), encrypted secrets, audit trail.
- Agent CRUD with immutable versions, import/export YAML, activation lifecycle.
- Manual executions with idempotency, concurrency/rate limits, persisted events and live
  SSE streaming with replay.
- Mock harness executed **in the worker**, with cancellation and timeouts.
- **OpenCode harness executes real tasks** in a separate worker image, tested against
  OpenCode v2.3.0: NDJSON event streaming, tool events, session capture, cancellation,
  usage tokens and `AGENTS.md` instruction isolation.
- **Pi harness executes real tasks** in a separate worker image, tested against Pi v1.5.2:
  JSON event streaming, session capture/resume, read-only tool allowlist.
- **X read connector**: X recent-search posts are hydrated into the harness input as
  untrusted context (bearer token from an encrypted connection).
- **GitHub read/write via the policy-enforcing tool broker** (OpenCode/Pi → MCP → broker → GitHub),
  verified end-to-end; write capabilities are approval-gated.
- **Agent Teams**: pipeline + native subagent modes, durable work queue, dispatcher with
  auto-claim, close-the-loop (labels/comments), multi-repo workspace.
- **Approvals UI**, worker heartbeats + orphaned-execution reaper, and opt-in
  **per-execution Docker containers** for local/single-user deployments.
- **Kubernetes Jobs mode**: per-execution container isolation via K8s Jobs.
- **Memory**: short-term and summary memory for persistent agents.
- **OIDC**: configurable from the Settings UI, JWT validation against provider JWKS.
- **Custom CA certificates**: Helm chart support for private CAs.
- **Dark/light mode**: theme toggle in the web app.
- Providers (custom OpenAI-compatible endpoints, harness-managed), connectors,
  connections, triggers, dashboard, API tokens, health/readiness/metrics, OpenAPI 3.1 +
  Swagger UI.
- Next.js mobile-first web app (dashboard, Agent Studio, executions with live logs,
  harnesses, models, connectors, skills, workers, settings, work wizard) with PWA assets.
- Docker images (non-root, reproducible) and a Helm chart deployed and smoke-tested on a
  local Kubernetes cluster with ingress host `orbit.example.com`.
- **GitHub Actions CI**: tests, lint, security scan (Trivy, cargo audit, pnpm audit,
  gitleaks), SBOM generation, container image builds.

Not yet implemented — do not assume otherwise:

- GitHub **writes** still need live end-to-end verification (approvals UI is in place).
- Claude Code adapter (registered descriptor only, fail closed).
- X posting, OAuth flows, webhook verification (Phase 2).
- Enterprise RBAC, shared connections, quotas, marketplace (Phase 4).
- Agent workflow branching/parallel execution (issue #9).
- Large-scale agent simulation with result aggregation (issue #11).

See `CHANGELOG.md` for the detailed list and `docs/` for architecture and decisions.

## Quick start (Docker Compose)

```bash
cp .env.example .env          # adjust secrets for anything beyond local dev
docker compose -f deploy/compose/compose.yaml up --build
```

- Web UI: http://localhost:3000
- API: http://localhost:8080/api/v1 · docs: http://localhost:8080/docs
- Postgres: `localhost:5432` (`orbit`/`orbit`, db `orbit`)
- NATS: `localhost:4222`, monitoring `http://localhost:8222`

The stack seeds demo agents (mock harness). Open an agent → **Run agent** → watch the
live execution timeline.

## OpenCode + X quick start

1. In **Connections**, add an X connection with a bearer token and optional
   `config.query` (for example `{"query":"#ORBIT"}`), selecting `posts.search`.
2. Create an agent with harness **OpenCode**, a harness-managed model
   (`opencode-go/<model>`), and the X connector bound to that connection with
   `posts.search`.
3. Run it with a task such as "Summarize recent X posts" and optionally
   `{"query":"#ORBIT"}` as payload.

OpenCode credentials are provided to the execution plane via an `opencode auth export`
Secret (see `docs/development/deployment.md`); credentials never pass through prompts or
the browser.

## Kubernetes (Helm)

The chart targets the example.com private registry and ingress conventions:

```bash
# build + push linux/amd64 images to the private registry
TAG=0.1.0 ./scripts/build-images.sh
TAG=0.1.0 ./scripts/push-images.sh

# deploy (default host orbit.example.com)
helm upgrade --install orbit deploy/helm/orbit \
  --kube-context your-cluster \
  --namespace orbit --create-namespace \
  -f deploy/helm/orbit/values.yaml
```

`values.yaml` bundles PostgreSQL and NATS for test clusters and uses dev auth. For
production, start from `deploy/helm/orbit/values-prod.example.yaml` (external DB/NATS,
OIDC, TLS, externally managed secrets, pinned tags).

Ingress routes: `/docs`, `/healthz`, `/readyz` → API; `/api` → web (same-origin proxy
that injects the dev token server-side or forwards OIDC/API bearer headers); `/` → web.
SSE-friendly annotations are included (`proxy-buffering: off`, long read/send timeouts).

Dev auth mode is enabled by default for local/test deployments. The web proxy injects
the dev token server-side, so do not expose a dev-auth deployment beyond your LAN
until OIDC is configured.

## Native development

```bash
docker compose -f deploy/compose/compose.dev.yaml up -d
cp .env.example .env

cargo run -p orbit-api-server      # terminal 1
cargo run -p orbit-worker          # terminal 2
pnpm install
pnpm --filter @orbit/web dev       # terminal 3 (proxies /api to localhost:8080)
```

## Tests

```bash
cargo test --workspace
ORBIT_TEST_DATABASE_URL=postgres://orbit:orbit@localhost:5432/orbit cargo test --workspace
pnpm --filter @orbit/web typecheck && pnpm --filter @orbit/web build
helm lint deploy/helm/orbit
helm template orbit deploy/helm/orbit -n orbit | kubectl apply --dry-run=client -f -
./scripts/smoke-k8s.sh http://localhost:8080
```

## Repository layout

```
apps/web/                 Next.js web app (App Router, Tailwind, shadcn-style UI)
packages/api-client/      OpenAPI-generated typed client
packages/contracts/       Shared Zod schemas (web + future Expo app)
services/api|worker/      Rust binaries
crates/orbit-*/           domain · contracts · store · events · security · harness · api
deploy/docker/            Dockerfiles
deploy/compose/           Local stacks
deploy/helm/orbit/        Helm chart
docs/                     Architecture, ADRs, development and deployment guides
```

## Security

- Autonomy levels 1–4 are enforced by the platform, not by prompts.
- Provider/connector credentials are AES-256-GCM encrypted at rest with a master key
  (`ORBIT_MASTER_KEY`), write-only through the API, and never logged or serialized.
- The control plane never executes agent tasks; execution belongs to the worker and,
  in production, isolated harness containers/Jobs.
- External content is untrusted: trusted system instructions are transported separately
  with provenance metadata.
- Never commit real credentials; `.env` is ignored and the example key is development-only.

## Documentation

- `docs/architecture/overview.md` — layers, modules, flows
- `docs/architecture/domain-model.md` — entities and invariants
- `docs/architecture/harness-contract.md` — adapter contract and implementation status
- `docs/architecture/provider-connector-contract.md`
- `docs/architecture/api.md` — REST/OpenAPI/SSE/auth conventions
- `docs/architecture/events.md` — event bus, durability, triggers
- `docs/decisions/` — ADRs
- `docs/development/local.md`, `docs/development/testing.md`, `docs/development/deployment.md`
- `AGENTS.md` — contributor and AI-agent operating manual

## License

Apache-2.0.
