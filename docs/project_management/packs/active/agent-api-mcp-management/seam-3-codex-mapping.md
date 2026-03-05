# SEAM-3 — Codex backend mapping

- **Name**: Codex MCP management mapping
- **Type**: platform (backend mapping)
- **Goal / user value**: Implement the universal MCP management operations for the Codex built-in backend by mapping requests
  to `codex mcp add/get/list/remove` with pinned output bounds and process context.

## Scope

### In

- Implement `AgentWrapperBackend::{mcp_list,mcp_get,mcp_add,mcp_remove}` for the Codex backend.
- Map universal requests to Codex CLI semantics (pinned by CLI manifest snapshot):
  - `list` → `codex mcp list --json`
  - `get` → `codex mcp get --json <name>`
  - `remove` → `codex mcp remove <name>`
  - `add`:
    - `Stdio` → `codex mcp add <name> [--env KEY=VALUE]* -- <command...>`
    - `Url` → `codex mcp add <name> --url <url> [--bearer-token-env-var ENV_VAR]`
- Ensure command execution honors `context.{working_dir,timeout,env}` and output bounds.

### Out

- Universalizing Codex-only MCP commands (`mcp login/logout`).

## Primary interfaces (contracts)

### Inputs

- `AgentWrapperMcp*Request` types (SEAM-1)

### Outputs

- `AgentWrapperMcpCommandOutput` (bounded stdout/stderr; truncation markers)

## Key invariants / rules

- Must not emit stdout/stderr as run events.
- Must not mutate parent env; request env overrides apply only to spawned Codex process.
- `add/remove` support must respect write enablement and capability advertising (SEAM-2).

## Dependencies

- **Blocks**:
  - SEAM-5 (tests pin mapping behavior)
- **Blocked by**:
  - SEAM-1 (types + hooks + bounds)
  - SEAM-2 (write enablement + isolated homes, for `add/remove`)

## Touch surface

- `crates/agent_api/src/backends/codex.rs`
- Wrapper surfaces (if gaps exist for context/timeout/env or home isolation):
  - `crates/codex/src/commands/mcp.rs`
  - `crates/codex/src/cli/mcp.rs`

## Verification

- Unit tests for request validation and correct argv construction (especially `add` transport mapping).
- Integration tests (opt-in if needed) that run against an isolated home and assert add/remove changes are localized.

## Risks / unknowns

- None (pinned: Codex `list/get` always pass `--json` for deterministic machine-friendly output; v1 still does not require
  cross-backend output parity).

## Rollout / safety

- `add/remove` capabilities remain disabled by default and only become reachable under explicit enablement (SEAM-2).
