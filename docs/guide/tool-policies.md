# Tool policies

Tool policies add per-call `allow | require_approval | deny` decisions on top of autonomy
and capability grants. They can only make a decision stricter; a license or policy can
never grant a capability.

## Rules

`GET/POST /api/v1/tool-policies`, `PATCH/DELETE /api/v1/tool-policies/{id}` (settings
write). Rules are scoped to the org and optionally an agent; agent rules extend org rules.

| Matcher | Example |
|---|---|
| `tool` glob | `Bash`, `Read`, `github_*` |
| `capability` glob | `issues.*` |
| `effect` | `read`, `write`, `external_side_effect` |
| `connection` glob | `github-prod` |
| `pathGlob` | `secrets/**`, `src/**` |
| `commandRegex` | `^rm -rf`, `^git push` |

All configured fields must match. Empty matchers match everything (use for a default
`require_approval`).

## Precedence

Platform deny → rule deny → require_approval → allow. An `allow` rule cannot bypass a
platform approval or denial. Native harness tools (no connector capability) default to
`require_approval`; an explicit `allow` rule can make a specific safe call unattended.

## Enforcement

The worker broker evaluates rules for connector tools and exposes `POST /policy` for
native hooks. Approvals are durable, scoped to the execution, expire with it, and record
the rule, reason, impact and arguments. Every deny/approval is audited and appears as a
`tool_call` execution event. See `docs/architecture/runtime-contract.md` and the launcher docs.
