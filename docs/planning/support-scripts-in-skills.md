# Support Scripts in OpenCode Skills — Plan

Status: proposed
Scope: Skill materialization, security, and harness integration

## Current state

- Skills are stored as `SKILL.md` content + a `files` map (path → string content)
- The worker's `skills::materialize()` writes `SKILL.md` and all files to `.pi/skills/{slug}/` and `.opencode/skills/{slug}/`
- Files are validated: relative paths only, no `..`, no backslashes, max 50 files, string content
- **Scripts are already supported as files** — a skill can include `scripts/run.sh` in its `files` map
- **The gap**: scripts are written but not made executable (`chmod +x`), and there is no validation for script extensions or security

## Problem

1. **No execute permission** — scripts written to the workspace are not `chmod +x`'d, so the harness cannot run them
2. **No script validation** — any file extension is allowed, including potentially dangerous ones
3. **No size limits** — large scripts could be written without restriction
4. **No discovery** — OpenCode doesn't know which files are scripts vs references

## Plan

### Phase 1: Worker materialization (chmod +x)

**File:** `services/worker/src/skills.rs`

1. After writing each file, check if it's in a `scripts/` directory
2. If so, `chmod +x` the file (Unix) or set executable permissions
3. Log which scripts were materialized

```rust
// After writing file:
if path.starts_with("scripts/") || path.contains("/scripts/") {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&target).await?.permissions();
        perms.set_mode(0o755);
        tokio::fs::set_permissions(&target, perms).await?;
    }
}
```

### Phase 2: API validation (script safety)

**File:** `crates/orbit-api/src/handlers/skills.rs`

1. Define allowed script extensions: `.sh`, `.bash`, `.py`, `.js`, `.ts`, `.rb`, `.pl`
2. Validate that files in `scripts/` directories have allowed extensions
3. Add per-file size limit (e.g. 100KB per script, 1MB total for scripts)
4. Reject scripts with shebangs that don't match the extension
5. Add `scripts` count to the skill validation report

### Phase 3: OpenCode integration

**File:** `crates/orbit-harness/src/opencode.rs`

1. OpenCode discovers scripts via the skill directory — no code change needed
2. The `opencode.json` MCP config already points to the skill directory
3. Add a note in the trusted instructions: "Scripts are available in `.opencode/skills/{slug}/scripts/`"
4. Ensure the worker's `PWD` is set to the workspace (already done)

### Phase 4: UI (skill editor)

**File:** `apps/web/app/skills/page.tsx`

1. Add a "Scripts" section to the skill editor
2. File upload or textarea for each script
3. Show script names with execute badge
4. Validate script extensions client-side

### Phase 5: Security hardening

1. **Path traversal**: Already validated (no `..`, no `/` prefix, no backslashes)
2. **Extension allowlist**: Only allow known script extensions
3. **Size limits**: Per-file (100KB) and per-skill (1MB for scripts)
4. **No binary content**: Validate that script content is valid UTF-8
5. **Audit**: Log when scripts are materialized and executed

### Phase 6: Testing

1. Unit test: materialize writes scripts with execute permission
2. Integration test: skill with script can be created and retrieved
3. E2E test: OpenCode execution with a skill that uses a script

## Files to change

| File | Change |
|---|---|
| `services/worker/src/skills.rs` | chmod +x for scripts |
| `crates/orbit-api/src/handlers/skills.rs` | Script validation (extensions, size limits) |
| `crates/orbit-harness/src/opencode.rs` | Add script path hint to trusted instructions |
| `apps/web/app/skills/page.tsx` | Scripts section in skill editor |
| `crates/orbit-domain/src/agent.rs` | Add script validation to skill definition (if applicable) |

## Effort estimate

- Phase 1 (chmod): 30 min
- Phase 2 (validation): 1 hour
- Phase 3 (OpenCode): 30 min
- Phase 4 (UI): 2 hours
- Phase 5 (security): 1 hour
- Phase 6 (testing): 1 hour

Total: ~6 hours

## Open questions

1. Should scripts be allowed to call external binaries (git, curl, etc.)?
2. Should there be a sandbox for script execution (containers, seccomp)?
3. Should script output be captured and returned to the agent?
EOF
echo done