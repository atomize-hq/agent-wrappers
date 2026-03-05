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
  - `get` → `claude mcp get <name>` (**win32-x64 only** in the pinned Claude Code CLI manifest)
  - `remove` → `claude mcp remove <name>` (**win32-x64 only**; also gated by write enablement in SEAM-2)
  - `add` (**win32-x64 only**; gated by write enablement in SEAM-2):
    - `Stdio` → `claude mcp add [--transport stdio] [--env KEY=value]* <name> <command> [args...]`
    - `Url`:
      - when `bearer_token_env_var == None` → `claude mcp add --transport http <name> <url>`
      - when `bearer_token_env_var == Some(_)` → reject as `InvalidRequest` (pinned; no deterministic/safe mapping to `--header` in v1)
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
  - Note: for the pinned Claude Code CLI manifest, `mcp add/get/remove` are **win32-x64 only**; tests should be
    target-aware (or skip on unsupported targets).

## Risks / unknowns

- None (pinned: `Url.bearer_token_env_var` is rejected as `InvalidRequest` for Claude in v1).

- **Platform availability**: the pinned Claude Code CLI manifest snapshot shows `mcp add/get/remove` only on `win32-x64`.
  Treat this as authoritative for v1: on unsupported targets, the Claude backend MUST NOT advertise
  `agent_api.tools.mcp.{get,add,remove}.v1` and MUST fail-closed with `UnsupportedCapability` when invoked.

## Rollout / safety

- `add/remove` capabilities remain disabled by default and only become reachable under explicit enablement (SEAM-2).
