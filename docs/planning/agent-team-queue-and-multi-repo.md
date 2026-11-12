# Agent Team queue and multi-repo plan

Status: proposed / implementation in progress
Scope: ORBIT Agent Teams (not Pi-specific; Pi uses pipeline mode only)

## Current baseline

- Agent Teams: create/edit/run, native OpenCode subagents, pipeline mode.
- Skills: team-level and execution-level.
- Queue: schema, API, UI, manual add, run one, run pending.
- Auto-launch: single-execution worker per step.
- Connectors: GitHub and GitLab capabilities.
- Missing: automatic issue discovery/claiming and multi-repo workspaces.

---

## Phase 1 — Work source configuration

Goal: an Agent Team knows where to find work.

Add to the team definition:

```json
{
  "workSource": {
    "provider": "github",
    "connectionId": "…",
    "repository": "opsifyhq/orbit",
    "labels": ["agent-ready"],
    "claimLabel": "agent-in-progress",
    "doneLabel": "agent-done",
    "failedLabel": "agent-failed",
    "maxItems": 5,
    "pollSeconds": 60,
    "autoRun": true
  }
}
```

GitLab variant:

```json
{
  "provider": "gitlab",
  "connectionId": "…",
  "groupId": "opsifyhq",
  "project": "opsifyhq/orbit",
  "labels": ["agent-ready"],
  "boardId": 123
}
```

Deliverables:

- Team create/edit validation.
- UI: Work source section.
- Enabled/disabled toggle.
- Connection and provider selection.
- Label/project/group fields.

Acceptance:

- Save and reload work source.
- Cannot enable with a missing connection or target.
- Existing teams without a work source remain manual.

---

## Phase 2 — Dispatcher

Goal: turn matching issues into queue items automatically.

Where: `orbit-api` background task, per-team DB lock, read-only connector client.

Loop:

```text
for each enabled work source:
  acquire per-team advisory lock
  list open issues matching labels
  skip issues already in queue
  claim issue: remove agent-ready, add agent-in-progress
  insert into team_queue_items
  if autoRun, start the queue item
  release lock
```

Deliverables:

- Read-only GitHub/GitLab issue listing from the API using the encrypted connection.
- Claim via label update.
- Dedupe by `(team_id, external_key)`.
- `autoRun` integration with the queue item runner.
- Last poll / last error fields.
- Max items per tick.
- Rate-limit handling.

Acceptance:

- Two API replicas do not double-dispatch the same issue.
- Re-polling the same issue does not create a second queue item.
- Claim failure skips without creating a queue item.
- Disabling the work source stops new enqueues.

---

## Phase 3 — Close the loop

Goal: issue status reflects execution outcome.

On success:

- Comment with PR/MR URL.
- Remove `agent-in-progress`.
- Add `agent-done`.
- Queue item `succeeded`.

On failure:

- Remove `agent-in-progress`.
- Add `agent-failed`.
- Store error.
- Optional retry limit.

On cancellation:

- Remove `agent-in-progress`.
- Return item to `pending` or mark `cancelled`.

Deliverables:

- Queue-to-TeamRun outcome watcher.
- Label/comment calls through the connector.
- Retry policy.
- UI actions: retry, cancel, requeue.

Acceptance:

- No item remains `running` after its TeamRun terminates.
- Failed issue never remains claimed as `agent-in-progress`.
- Successful item has a PR/MR link.

---

## Phase 4 — Multi-repo workspace

Goal: one team can change project A and project B.

Replace `repository` with:

```json
{
  "repositories": [
    { "connectionId": "…", "repository": "opsifyhq/app", "path": "app", "primary": true },
    { "connectionId": "…", "repository": "opsifyhq/lib", "path": "lib" }
  ]
}
```

Workspace:

```text
/tmp/orbit-workspaces/<execution>/
  .opencode/
  AGENTS.md
  app/.git
  lib/.git
```

For a single repository, preserve the current root layout for compatibility.

Deliverables:

- Team/run repository list.
- Sequential checkout per repository.
- Path traversal validation.
- Multi-repo manifest in the prompt.
- Git tools accept repository/path:
  - `repo.commit(repository, message)`
  - `repo.push(repository, branch)`
  - `pulls.create(repository, …)`
  - `merge_requests.create(project, …)`
- Per-repo credentials/connection.
- UI repository editor and status.

Acceptance:

- Builder edits both repos.
- Commit/push targets the correct repo.
- Two PRs/MRs are created and linked in the issue.
- A failure in one repo does not silently report overall success.
- No path can escape the workspace.

---

## Phase 5 — Completion policy

Goal: verify “done” instead of trusting the agent.

Team completion config:

```json
{
  "completion": {
    "requirePullRequests": true,
    "requireChecksGreen": true,
    "requireReview": false
  }
}
```

TeamRun only succeeds when:

- required PR/MR exists,
- checks/pipelines are green,
- optionally review approved.

Deliverables:

- Connector checks/pipeline verification.
- TeamRun state `awaiting_checks`.
- UI evidence: PR, checks, review.

---

## Phase 6 — UI

Agent Teams page:

- Work source card.
- Queue with statuses.
- Run all pending.
- Per-item run/retry/cancel.
- Last poll/error.
- Multi-repo list.
- Completion evidence.

Executions page:

- Group by TeamRun and queue item.
- PR/MR and check status per item.

---

## Phase 7 — Testing and release

Tests:

- Queue dedupe.
- Dispatcher claim race.
- API replica double-polling.
- TeamRun failure releases claim.
- Multi-repo checkout/commit/push.
- Path traversal rejection.
- GitLab group board project mapping.

Deploy:

- Migration.
- API/worker/web images.
- Helm rollout.
- GitHub + GitLab live E2E.
- Tag patch release.

---

## Recommended order

```text
1. Work source config + UI
2. Dispatcher + claim/dedupe
3. Close-the-loop labels/comments
4. Manual batch testing
5. Multi-repo workspace
6. Per-repo git/PR tools
7. Completion policy
8. Live GitHub/GitLab E2E
9. Release
```

The smallest useful milestone is 1–3: automatic issue pickup, queueing, claim, run, and issue result. Multi-repo can follow without changing the dispatcher model.

Pi note: Pi does not have the OpenCode native primary/subagent mechanism in the tested 1.5.x release. Pi can participate in pipeline teams only; this plan is harness-agnostic unless native subagents are explicitly selected.
