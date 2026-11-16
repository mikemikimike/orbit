# Skills

A skill is a reusable `SKILL.md` package stored in the database and materialized into a
worker workspace for an execution.

## Shape

- `slug` (unique per org), `name`, `description`, markdown `content`
- `files`: path → text, including optional `scripts/*`
- Monotonic `version`

## Materialization

At execution start the worker copies assigned skills (agent-level, execution-level and
worker-profile skills) into the workspace, makes validated script files executable, and
references them in the prompt. Script allowlisting rejects unsupported extensions and
path traversal. Skills never run with control-plane credentials.

## Assignment

Attach skills to an agent, a team, an execution request or a worker profile. Agent Studio
and the Work wizard show the effective set. Skill content is trusted configuration; task
input remains untrusted and is kept separate from instructions.
