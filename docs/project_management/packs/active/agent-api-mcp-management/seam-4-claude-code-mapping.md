# SEAM-4 — Claude Code backend mapping

- **Name**: Claude Code MCP management mapping
- **Type**: platform (backend mapping)
- **Goal / user value**: Implement the universal MCP management operations for the Claude Code built-in backend by mapping
  requests to `claude mcp add/get/list/remove` with pinned output bounds and process context.

## Scope

### In

- Implement `AgentWrapperBackend::{mcp_list,mcp_get,mcp_add,mcp_remove}` for the Claude Code backend.
- Map universal requests to Claude CLI semantics (pinned by CLI manifest snapshot):
  - `list` → `claude mcp list`
  - `get` → `claude mcp get <name>`
  - `remove` → `claude mcp remove <name>`
  - `add`:
    - `Stdio` → `claude mcp add --transport stdio [--env KEY=value]* <name> <command> [args...]`
    - `Url` → `claude mcp add --transport http [--header ...]* <name> <url>` (auth/header mapping pinned below)
- Ensure command execution honors `context.{working_dir,timeout,env}` and output bounds.

### Out

- Universalizing Claude-only MCP commands (`add-json`, `add-from-claude-desktop`, `serve`, `reset-project-choices`, etc.).

## Primary interfaces (contracts)

### Inputs

- `AgentWrapperMcp*Request` types (SEAM-1)

### Outputs

- `AgentWrapperMcpCommandOutput` (bounded stdout/stderr; truncation markers)

## Key invariants / rules

- Must not emit stdout/stderr as run events.
- Must not mutate parent env; request env overrides apply only to spawned Claude process.
- `add/remove` support must respect write enablement and capability advertising (SEAM-2).

## Dependencies

- **Blocks**:
  - SEAM-5 (tests pin mapping behavior)
- **Blocked by**:
  - SEAM-1 (types + hooks + bounds)
  - SEAM-2 (write enablement + isolated homes, for `add/remove`)

## Touch surface

- `crates/agent_api/src/backends/claude_code.rs`
- Wrapper surfaces (if gaps exist for context/timeout/env or home isolation):
  - `crates/claude_code/src/commands/mcp.rs`
  - `crates/claude_code/src/client/mod.rs`

## Verification

- Unit tests for request validation and correct argv construction (especially `add` mapping).
- Integration tests (opt-in if needed) that run against an isolated home and assert add/remove changes are localized.

## Risks / unknowns

- **Bearer token env var mapping**: universal type includes `bearer_token_env_var`, but Claude’s CLI surface appears to expose
  `--header` rather than an env-var-name flag. Decide and pin one of:
  - a Claude-specific convention for `bearer_token_env_var` (e.g., a deterministic header expansion rule), or
  - reject `bearer_token_env_var` as `InvalidRequest` for Claude until upstream supports it directly.
  - **De-risk plan**: resolve in SEAM-4 before SEAM-5 integration tests land.

- **Manifest omissions**: the CLI manifest snapshot indicates `mcp add/get/remove` only on a subset of targets, but the wrapper
  crate already models these commands; confirm this is a snapshot artifact (not a real product constraint) and avoid assuming
  platform asymmetry in `agent_api` behavior.

## Rollout / safety

- `add/remove` capabilities remain disabled by default and only become reachable under explicit enablement (SEAM-2).
