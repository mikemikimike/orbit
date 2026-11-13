# Support Different Models per Agent in Agent Teams — Plan

Status: proposed
Issue: #12
Scope: Agent Teams (pipeline + native modes)

## Problem

In an Agent Team, each agent (primary + subagents) may need a different model.
Currently the team definition has a single model field (or none), and all steps
use the primary agent's model. There's no way to specify "use a cheaper model for
the reviewer step" or "use a code-specialized model for the builder step".

## Current state

### TeamDefinition
```rust
pub struct TeamDefinition {
    pub mode: String,
    pub instructions: String,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub work_source: Option<WorkSourceDefinition>,
    pub repositories: Vec<RepositorySpec>,
    pub worker_profile_id: Option<Uuid>,
    pub auto_launch: bool,
    pub skills: Vec<String>,
    pub steps: Vec<TeamStepDefinition>,
}

pub struct TeamStepDefinition {
    pub agent_id: Uuid,
}
```

### Model resolution in start_step()
The worker resolves the model from the agent's spec:
- If `model.management == Harness`: uses harness-managed provider (e.g. opencode-go)
- If `model.management == Platform`: uses ORBIT-managed provider (e.g. vLLM)

The agent already defines its own model in its spec. So in pipeline mode, each
step naturally uses the agent's own model because `start_step()` calls
`state.store.get_agent()` and `state.store.get_current_version()` for that step's
agent, then reads `spec.model`.

### The actual gap

1. **Team-level model override**: The team definition doesn't have a `model` field
   at all. There's no way to override the model for all steps or specific steps.

2. **Native mode subagents**: In native mode, subagents are materialized as
   OpenCode subagent configs. The `native_subagents()` function builds the subagent
   config but doesn't include the model field. So subagents use the primary's model.

3. **OpenCode subagent model**: OpenCode subagents can specify their own model in
   the agent definition. ORBIT doesn't pass this through.

## Plan

### Phase 1: Use agent's own model (already works for pipeline mode)

In pipeline mode, each step calls `get_agent()` and `get_current_version()` for
that step's agent, then reads `spec.model`. This means **each agent already uses
its own model**. No code change needed for this — it's already how it works.

Verification: test with two agents using different models in a pipeline team.

### Phase 2: Team-level model override

Add an optional `model` field to `TeamDefinition`:

```rust
pub struct TeamDefinition {
    // ... existing fields ...
    #[serde(default)]
    pub model: Option<TeamModelOverride>,
}

pub struct TeamModelOverride {
    /// Provider name (harness-managed or ORBIT-managed)
    pub provider: Option<String>,
    /// Model id
    pub model: Option<String>,
    /// Management mode
    pub management: Option<String>,
}
```

When set, override the agent's model for all steps. When not set, use the agent's
own model (Phase 1 behavior).

In `start_step()`, after resolving the agent's spec, check if the team has a
model override. If so, override `spec.model` before creating the execution.

### Phase 3: Per-step model override

Add an optional `model` field to `TeamStepDefinition`:

```rust
pub struct TeamStepDefinition {
    pub agent_id: Uuid,
    #[serde(default)]
    pub model: Option<TeamModelOverride>,
}
```

When set, override the model for that specific step. Per-step takes precedence
over team-level.

### Phase 4: Native mode subagent models

Update `native_subagents()` to include the model from each agent's spec:

```rust
let model = match (&spec.model.provider, &spec.model.model) {
    (Some(provider), Some(model)) if !model.contains('/') => {
        Some(format!("{provider}/{model}"))
    }
    (_, Some(model)) => Some(model.clone()),
    _ => None,
};
subagents.push(serde_json::json!({
    "name": agent.name,
    "slug": agent.slug,
    "model": model,  // <-- already there!
    ...
}));
```

This already works! The model is already passed to OpenCode subagents.

### Phase 5: UI

- Show each agent's model in the team editor step list
- Add a "Model override" option per step (optional)
- Add a team-level model override in the team settings
- Display model per step in the team run detail view

## Files to change

| File | Change |
|---|---|
| `crates/orbit-api/src/handlers/teams.rs` | Add `model` field to `TeamDefinition` and `TeamStepDefinition` |
| `crates/orbit-api/src/handlers/teams.rs` | Apply model override in `start_step()` |
| `apps/web/app/agent-teams/page.tsx` | Show model per agent, add override UI |
| `apps/web/lib/types.ts` | Add `model` to team definition type |

## Effort estimate

- Phase 1 (verify): 30 min
- Phase 2 (team-level override): 1 hour
- Phase 3 (per-step override): 1 hour
- Phase 4 (native mode): already done
- Phase 5 (UI): 2 hours

Total: ~4.5 hours

## Open questions

1. Should model override be harness-managed, platform, or both?
2. Should the override replace the agent's model entirely, or merge (e.g. override model but keep provider)?
3. How to handle model overrides in the Work wizard (already passes harness_override)?
EOF
echo done