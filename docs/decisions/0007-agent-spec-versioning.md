# ADR-0007: Immutable, validated agent specs

**Status:** Accepted

## Context
Agents must be reproducible: an execution must be explainable months later, and config
drift between edits must not mutate history.

## Decision
- `AgentSpec` v1 (`apiVersion: orbit.io/v1`, `kind: Agent`) with camelCase fields.
- Every configuration change creates a new immutable `agent_versions` row holding
  canonical JSON (`spec`), its SHA-256 (`checksum`), and the source YAML.
- `agents.current_version` points to the active version; executions reference the exact
  `agent_version_id`.
- Validation happens in `orbit-domain` (semantic rules) before persistence; the API maps
  failures to `validation_error` with field paths.
- Runtime-specific configuration lives under `spec.runtime.config` and is only validated
  by the selected adapter, keeping the core schema runtime-agnostic.

## Consequences
- Rollback = point `current_version` at an earlier version.
- Duplicate specs are rejected per agent (unique checksum) to avoid noise.
