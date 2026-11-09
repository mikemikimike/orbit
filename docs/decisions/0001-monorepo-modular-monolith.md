# ADR-0001: Monorepo with a modular monolith control plane

**Status:** Accepted (Phase 0)

## Context
ORBIT needs a fast-moving MVP spanning a web app, a Rust API, runtime adapters and
deployment artifacts, plus a future mobile app that must share contracts.

## Decision
- Single repository with pnpm workspaces (`apps/*`, `packages/*`) and a Cargo workspace
  (`crates/*`, `services/*`).
- The control plane is one deployable binary (`orbit-api`) plus one worker binary
  (`orbit-worker`); domains are separated by crates and module boundaries, not by
  network hops.
- Shared contracts live in Rust (`orbit-contracts`) and TypeScript (`packages/*`),
  generated from a single OpenAPI document for HTTP types.

## Consequences
- One build/test pipeline; contract changes are atomic across frontend and backend.
- Microservices can be extracted later along existing crate boundaries.
- The worker is independently scalable and is the only place runtime adapters run.
