# Testing Strategy

| Layer | Tooling | What is covered |
|---|---|---|
| Domain | `cargo test` (pure) | spec validation, autonomy policy matrix, state transitions, pagination encoding |
| Harness contracts | `cargo test` | every registered adapter passes the shared capability/contract suite; mock harness streaming + cancellation |
| Store | `cargo test` with `ORBIT_TEST_DATABASE_URL` | CRUD, org scoping, optimistic version conflict, execution claim compare-and-set, idempotency |
| API | `axum` router + `tower::ServiceExt::oneshot` | auth extractor, error envelope, pagination, agent lifecycle, execution creation, SSE replay |
| Helm | `helm lint`, `helm template` | chart validity, required env wiring, probes, external PG/NATS |
| Frontend | `tsc --noEmit`, `next build` | type-safe API usage across every page and build-time correctness |
| End-to-end smoke | `scripts/smoke-k8s.sh` | Kubernetes stack: create agent → execute → stream events → terminal status |
| UI acceptance | `docs/development/ui-test-plan.md` | dashboard, Studio, executions/SSE, credentials, OpenCode, connectors, approvals |

DB-backed tests are skipped unless `ORBIT_TEST_DATABASE_URL` is set, so `cargo test`
stays usable without infrastructure. CI sets it to a Postgres service.

Harness integrations are only considered "working" when exercised against the real
harness. The OpenCode adapter is **available** and tested against OpenCode v2.3.0.
The Pi adapter is **available** and tested against Pi v1.5.2.
Claude Code and generic adapters are registered as planned (fail closed).
