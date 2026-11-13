# ORBIT Documentation

## Architecture
- [Overview](architecture/overview.md) — layers, harness topology, component roles
- [Agent Teams](architecture/agent-teams.md) — pipeline, native subagents, queue, multi-repo
- [Runtime Contract](architecture/runtime-contract.md) — harness adapter trait, capabilities, errors
- [Diagram](architecture/diagram.md) — system context, execution sequence, broker policy path
- [Provider & Connector Contract](architecture/provider-connector-contract.md) — provider/connector specs

## Development
- [Local development](development/local.md) — running the stack locally
- [Deployment](development/deployment.md) — Docker Compose & Kubernetes
- [Kubernetes launcher](development/kubernetes-launcher.md) — worker launch auth (RBAC)
- [OIDC](development/oidc.md) — OIDC/SSO configuration
- [Testing](development/testing.md) — test expectations per change type

## Decisions
- [ADR index](decisions/) — architecture decision records

## Security
- [Threat Analysis](security/threat-analysis.md) — threat model, attack vectors, mitigations

## Planning
- [Agent Team queue & multi-repo](planning/agent-team-queue-and-multi-repo.md)
- [Provider UX & harness decoupling](planning/provider-ux-and-harness-decoupling.md)
- [Support scripts in skills](planning/support-scripts-in-skills.md)
- [Agent spawning for simulations](planning/agent-spawning-for-simulations.md)
- [Different models per agent teams](planning/different-models-per-agent-teams.md)
