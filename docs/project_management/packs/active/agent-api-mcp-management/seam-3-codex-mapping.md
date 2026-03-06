# SEAM-3 — Codex backend mapping

- **Name**: Codex MCP management mapping
- **Type**: platform (backend mapping)
- **Goal / user value**: Implement the universal MCP management operations for the Codex built-in backend by mapping requests
  to `codex mcp add/get/list/remove` with pinned output bounds and process context.

## Scope

### In

- Implement `AgentWrapperBackend::{mcp_list,mcp_get,mcp_add,mcp_remove}` for the Codex backend.
- Map universal requests to Codex CLI semantics as pinned in the canonical MCP management spec:
  - `docs/specs/universal-agent-api/mcp-management-spec.md` → “Built-in backend behavior” → “Built-in backend mappings (pinned)”
  - (target availability is still pinned by the CLI manifest snapshot)
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
- Manifest snapshot drift handling is pinned in the canonical spec: if runtime upstream behavior conflicts with the pinned
  CLI manifest snapshot, the operation MUST fail as `Err(AgentWrapperError::Backend { .. })` and MUST NOT silently mutate
  advertised capabilities at runtime (remediation is a follow-up repo update to manifests + mapping).

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
- Default integration tests (run under normal `cargo test` / `make test`) use hermetic fake binaries and isolated homes, and
  assert add/remove changes are localized (per the MCP management spec verification policy).
- Optional live smoke tests against real installed `codex` are opt-in (`#[ignore]` + `AGENT_API_MCP_LIVE=1` + configured binary
  path) and MUST NOT run in CI by default.

## Risks / unknowns

- None (pinned: Codex `list/get` always pass `--json` for deterministic machine-friendly output; v1 still does not require
  cross-backend output parity).

## Rollout / safety

- `add/remove` capabilities remain disabled by default and only become reachable under explicit enablement (SEAM-2).
