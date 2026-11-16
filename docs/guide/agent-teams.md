# Agent Teams

An Agent Team coordinates several agents for one outcome. Teams have two modes:

| Mode | How it runs |
|---|---|
| `pipeline` (default) | Every agent is a separate ORBIT execution, chained in order |
| `native` | The primary agent runs once and the remaining agents become harness-native subagents |

## Definition

- Ordered agents with a primary step, per-step instructions and an optional per-step model
  or model override
- Shared instructions, skills and repositories
- Optional **work source**: GitHub/GitLab provider, connection, labels, claim/done/failed
  labels, max items, poll interval and `autoRun`
- Optional queue completion policy

## Queue and dispatcher

The API background dispatcher polls enabled work sources under a per-team advisory lock:

```text
list matching issues → skip items already queued → claim label → insert queue item
→ optionally start the run → release lock
```

Queue items are deduplicated by `(team_id, external_key)`, so two API replicas cannot
double-dispatch. Manual add, run one, run pending, retry/re-queue and cancel are available
in the UI and API (`/api/v1/agent-teams/{id}/queue`). On success the issue gets a comment
with PR/MR links and the done label; failures apply the failed label or release the claim.

## Multi-repository workspace

A team can define `repositories[]` with `connectionId`, `path`, `primary` and `branch`.
Repositories are checked out sequentially (paths are validated against traversal), the
prompt gets a manifest, and git tools accept a `repository` argument. Multi-repo runs
operate on the worker workspace, not the control plane.

## Execution and limits

Team runs are executions (and count against the license's executions/day limit) and use
the same worker profiles, skills, tool policy and approvals as single agents.
