# Agent Teams and connector bindings

## Model

```text
Connection ──credential + config──► Agent connector binding ──capabilities──► Execution
                                           │
Agent Team ──ordered agents───────────────┘
     │
     └── TeamRun → one normal execution per agent, sequential
```

- **Connection**: one external account/credential, e.g. a GitHub token. It can be reused
  by many agents.
- **Agent connector binding**: the connector, `connectionId` and capability allowlist
  stored on the agent version. This is where a connection is attached.
- **Agent Team**: orchestration only. It has no connection field. Each step inherits the
  connections and capabilities of its agent.
- **Team skills and instruction**: shared by every step. Agent-level instructions define
  the role. The first agent is the primary agent; later agents are subagents.

## Attaching a connection

1. Create the credential under **Connections**.
2. Open each **Agent** in Agent Studio.
3. Enable the connector and select the same connection.
4. Select only the capabilities that agent needs.

Example for a GitHub coding team using one connection:

| Agent | Capabilities |
|---|---|
| Primary coder | `issues.list`, `issues.get`, `issues.update`, `repo.checkout` |
| Builder | `repo.checkout`, `repo.commit`, `repo.push`, `pulls.create` |
| Tester | `repo.checkout`, `checks.list` |
| Reviewer | `checks.list`, `pulls.get`, `pulls.list`, `issues.comment` |

The GitHub connection should set `defaultRepository` (and optionally
`allowedRepositories`). `repo.checkout` clones that repository before the harness starts.

## Minimal GitHub issue flow

1. Create a GitHub connection with a PAT that can read issues and push branches.
2. Create two agents:
   - **Primary / issue triage**: GitHub capabilities `issues.list`, `issues.get`,
     `issues.update`.
   - **Builder**: `repo.checkout`, `repo.commit`, `repo.push`, `pulls.create`.
3. Create a skill such as `github-issue-loop` that tells the agent:
   - list issues with label `agent-ready`,
   - choose one,
   - comment/assign/label it,
   - work only on that issue,
   - report the issue number and plan.
4. Create an **Agent Team** with the primary agent first and the builder second.
   Add the skill at team level.
5. Team instruction example:

   ```text
   Pick one open issue labelled agent-ready and fix it. Report the issue number,
   changed files, tests run and the branch. Do not pick another issue in this run.
   ```

6. Run the team. The primary lists/selects the issue; the builder receives the previous
   output, clones the repository, makes changes, commits and pushes.
7. Approve any write action according to the agents' autonomy settings.

## Current limitations

- Connections are **not** attached to an Agent Team. Attach them to each agent.
- A team run executes each agent **once**. A loop over many issues is either inside the
  primary agent/skill or multiple team runs; there is no team-level loop/queue yet.
- The run UI does not yet accept a repository override. Set `defaultRepository` on the
  connection.
- GitLab invocation, automatic per-execution worker launch and parallel subagents are
  still pending.
- GitHub writes require a PAT and may require approvals depending on autonomy.
