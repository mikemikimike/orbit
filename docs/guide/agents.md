# Agents

An agent is a versioned definition (`orbit.io/v1`) of *what* should run. It does not
contain an execution loop; a worker runs it through a harness adapter.

## Definition

| Field | Purpose |
|---|---|
| `metadata.name` / `description` / `labels` | Identity and grouping |
| `spec.harness.kind` | Default harness (`mock`, `opencode`, `pi`, future `claude_code`) |
| `spec.model` | Harness-managed provider/model or a platform `openai_compatible` provider |
| `spec.instructions` | Trusted system prompt, kept separate from untrusted input |
| `spec.runDefaults.task` | Optional task pre-filled for manual runs; never used as system instructions |
| `spec.connectors` | Connector bindings and the capabilities the agent may request |
| `spec.triggers` | Manual, schedule (cron), event or webhook |
| `spec.autonomy` | Level 1–4, approval mode and per-hour quota |
| `spec.resources` | Execution timeout, concurrency, max output tokens. Worker CPU/memory live on the worker profile |
| `spec.memory` | Whether the agent reads/writes durable memory |
| `spec.skills` | SKILL.md bundles materialized into the workspace |

## Autonomy

| Level | Behaviour |
|---|---|
| 1 Observe | Read-only capabilities |
| 2 Recommend | Analyse and propose; writes denied |
| 3 Prepare | Writes/side effects require approval |
| 4 Autonomous | Allowlisted writes execute without approval |

Approval is enforced by the platform (`evaluate_capability`), never by the prompt. The
tool policy layer can add argument-level restrictions and approvals on top
([tool-policies.md](tool-policies.md)).

## Lifecycle

Draft → Active → Paused → Archived. Editing creates a new immutable version; executions
pin the version they started with. Agent Studio shows the resolved harness/provider/model
and the execution limits. Run an agent from its page, the Work wizard, or an API/trigger.

## Harness vs agent

The harness is the execution environment, not the agent's identity. Work and team runs may
pass a `harnessOverride` for a run; otherwise the agent's stored harness is used.
Worker placement (`auto`, `local`, `docker`, `kubernetes`) is a routing constraint.
