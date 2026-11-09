# Testing Strategy

| Layer | Tooling | What is covered |
|---|---|---|
| Domain | `cargo test` (pure) | spec validation, autonomy policy matrix, state transitions, pagination encoding |
| Runtime contracts | `cargo test` | every registered adapter passes the shared capability/contract suite; mock runtime streaming + cancellation |
| Store | `cargo test` with `ORBIT_TEST_DATABASE_URL` | CRUD, org scoping, optimistic version conflict, execution claim compare-and-set, idempotency |
| API | `axum` router + `tower::ServiceExt::oneshot` | auth extractor, error envelope, pagination, agent lifecycle, execution creation, SSE replay |
| Helm | `helm lint`, `helm template` | chart validity, required env wiring, probes, external PG/NATS |
| Frontend | `tsc --noEmit`, `next build`, `vitest` | type-safe API usage, component rendering, form validation |
| End-to-end smoke | `scripts/smoke.sh` | compose stack: create agent → execute → stream events → terminal status |

DB-backed tests are skipped unless `ORBIT_TEST_DATABASE_URL` is set, so `cargo test`
stays usable without infrastructure. CI sets it to a Postgres service.

Runtime integrations are only considered "working" when exercised against the real
runtime. Phase 0's OpenCode adapter is explicitly marked *scaffolded* and is not claimed
as certified.
