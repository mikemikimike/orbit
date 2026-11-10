# ORBIT UI test plan

A practical, repeatable user flow for validating the web experience. Run it against the
fastest environment available:

- **Local Compose** — `http://localhost:3000` (recommended; see `docs/development/local.md`)
- **Kubernetes** — `http://orbit.mdk.local` (slower, but validates ingress/proxy/SSE)

Use a fresh database for the full flow (`docker compose ... down -v` locally). Dev auth is
enabled, so the browser needs no login; the web container injects the dev token server-side.

Time: 15–20 minutes for flows 1–5, plus 10 minutes for connector/runtime checks.

---

## Flow 1 — First-run dashboard (2 min)

| Step | Expected |
|---|---|
| Open `/` | Dashboard renders; four stat cards; recent executions or empty state |
| Check seeded agents | “Agent pulse” and agent cards show demo agents |
| Resize to 390×844 | Bottom navigation appears; no horizontal scrolling; cards stack |
| Click an agent card | Agent detail opens without a page error |

**Watch for:** console errors, skeleton that never resolves, `401/500` in the network tab.

---

## Flow 2 — Create, activate, and run a mock agent (6 min)

1. **Agents → New agent**
2. Identity: name `UI Smoke Agent`, description “UI test”.
3. Runtime: **Mock**; model management **Runtime**; leave provider/model empty.
4. Instructions: “You are a concise test agent. Summarize the task.”
5. Add a **Schedule** trigger (`*/30 * * * *`) and leave it enabled.
6. Autonomy: **L2 Recommend**; timeout 120s; concurrency 1.
7. **Create agent** → redirected to detail page.

Expected:
- Status `Draft`, version 1, trigger listed.
- No validation toast.

Then:
8. **Activate** → status `Active`.
9. **Run agent** → task “Summarize the current ORBIT deployment.” → **Queue execution**.
10. Execution page opens; status moves `queued → dispatched/running → succeeded`.
11. Timeline streams events (dispatch, runtime session, logs, output).
12. Output JSON is visible; usage shows tokens/time; agent link works.
13. Open **Executions** → the run appears; filter by `Succeeded` and `Running`.

---

## Flow 3 — Failure, cancellation, and live streaming (4 min)

1. On the agent, **Run agent** with a task containing `slow` → queue.
2. Execution page: status `running`; click **Cancel**.
3. Expected: `cancelled`, timeline shows the cancellation, no orphaned running state.
4. Run with a task containing `fail` → expected `failed` with an error block and error event.
5. While a run is active, refresh the execution page:
   - SSE reconnects, replays events from `seq 1`, then tails.
   - No duplicated events.

---

## Flow 4 — Versioning and editing (3 min)

1. **Edit** the agent → change instructions and autonomy to **L3 Prepare**.
2. **Save new version** → detail shows version 2.
3. **Versions** tab: v2 current, v1 preserved with checksum/timestamps.
4. Edit again and save the **same** configuration → expected conflict toast
   (“identical to an existing version”), no duplicate version.
5. **Configuration** tab: YAML matches the current version; **YAML** export downloads.

---

## Flow 5 — Credentials, providers, and audit (4 min)

### Models
1. **Models → Add provider**: name `Broken Gateway`, kind OpenAI-compatible,
   base URL `https://example.invalid/v1`, model `test`, credential `sk-test`.
2. Create → card shows `credential configured (encrypted)`; never the value.
3. **Test** → graceful failure toast (DNS/HTTP), not a crash.
4. Edit → replace credential; confirm it still never appears in the response or UI.

### Connections
5. **Connections → New connection**: connector **X**, credential `invalid-test-token`,
   capability `posts.search`, advanced config `{"query":"#orbit"}`.
6. Card shows `credential set`; no secret in the network payload.
7. Remove the connection afterwards.

### Settings
8. **Settings**: identity, org, permissions, platform/feature readiness.
9. **New token** → token shown once; copy works; list shows prefix only; revoke it.
10. Recent audit activity contains agent/execution/provider/token actions.

---

## Flow 6 — Real OpenCode runtime (5 min, optional)

Requires a host OpenCode worker (`make worker-opencode`) or the Kubernetes OpenCode
worker with imported auth.

1. Create an agent: runtime **OpenCode**, model management **Runtime**,
   provider `opencode-go`, model `deepseek-v4-pro`, timeout 240s.
2. Add a runtime config override if the UI exposes it:
   `{"autoApprove": true, "standalone": true, "model": "opencode-go/deepseek-v4-pro"}`.
3. Activate, run: “Reply with exactly: UI_OPENCODE_OK”.
4. Expected: `succeeded`, output text contains `UI_OPENCODE_OK`, session ref present,
   token usage > 0, timeline shows text/tool/usage events.
5. Run a long task and cancel → `cancelled`.

---

## Flow 7 — GitHub read via the tool broker (3 min, working locally)

Prerequisite: a host OpenCode worker (`make worker-opencode`) or the Kubernetes OpenCode
worker, plus a GitHub connection for a public repo (no credential needed for reads).

1. Create a GitHub connection with `{"allowedRepositories":["cli/cli"],"defaultRepository":"cli/cli"}`.
2. Create an agent: runtime OpenCode, GitHub connector, capability `issues.list`,
   autonomy L1.
3. Run: “Use tools.orbit.github_issues_list({owner:'cli',repo:'cli',state:'open'})
   and return 3 titles.”
4. Expected: `succeeded`; timeline shows the broker ready, the `issues.list` tool call,
   and completion. Reasoning/step/usage noise is hidden by default; click
   “show verbose” to reveal it.
5. `pwd` in a runtime shell must print `/tmp/orbit-workspaces/<execution-id>`, never the
   ORBIT repository directory.

## Flow 8 — Connector read path (X) (3 min)

1. Create an X connection with a **real** bearer token (or keep the invalid one for a
   negative test) and config `{"query":"#orbit"}`.
2. Create/bind an agent: X connector, connection selected, capability `posts.search`,
   autonomy L1/L2.
3. Run with task “Summarize recent posts” and optional payload `{"query":"#orbit"}`.
4. Expected with a valid token: execution succeeds and the timeline shows
   “Fetched N X post(s)”; output summarizes untrusted posts.
5. Expected with an invalid token: execution `failed` with a clear 401 message
   (“check bearer token/API plan”), not a silent empty result.

---

## Flow 9 — Approvals / GitHub writes (when approvals UI is available)

These are the acceptance steps for the broker milestone; the backend/API exists, the
approvals UI is still being finished.

1. Create a GitHub connection (fine-grained PAT or read-only public access) with
   `allowedRepositories` set to a test repo.
2. Agent: runtime OpenCode, GitHub connector, `issues.list` + `issues.comment`,
   autonomy **L3**.
3. Run “List open issues and propose a comment on #1”.
4. Expected: read tool succeeds; the write tool creates an approval and pauses.
5. Open **Approvals** (pending): approve with a comment.
6. Expected: comment appears on GitHub; execution continues; `tool.call.approved` audit.
7. Repeat and reject → write does not execute; agent receives a denied tool result.
8. Switch autonomy to L2 → writes are denied immediately (no approval prompt).
9. Switch to L4 with the write allowlisted → write executes without approval.

---

## Negative and security checks

- No secret value appears in HTML, JSON responses, execution events, logs, or tool output.
- Viewer-style access cannot mutate (when RBAC UI lands); dev mode is owner.
- Validation errors return field paths; the form highlights the section.
- Invalid cron, empty instructions, unknown runtime/provider are rejected.
- Idempotency: double-click **Queue execution** with the same key creates one execution.
- Deleting/archiving an agent preserves execution history by default.

## What to record

For each failure: page, action, expected vs actual, request id (response header
`x-request-id`), screenshot, and whether it reproduces after `down -v`.
