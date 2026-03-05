# SEAM-1 — MCP management contract + `agent_api` surface

- **Name**: MCP management contract + `agent_api::mcp` surface
- **Type**: integration (contract-definition)
- **Goal / user value**: Give orchestrators a backend-neutral, typed, capability-gated API to manage MCP server configs without
  depending on wrapper-specific crates.

## Scope

### In

- Implement the public `agent_api::mcp` module as pinned in:
  - `docs/specs/universal-agent-api/mcp-management-spec.md`
- Add `AgentWrapperGateway` convenience methods:
  - `mcp_list`, `mcp_get`, `mcp_add`, `mcp_remove`
- Add `AgentWrapperBackend` default hooks (non-breaking additive trait evolution) mirroring the `run_control` pattern:
  - `mcp_list`, `mcp_get`, `mcp_add`, `mcp_remove` (default: `UnsupportedCapability`)
- Implement request validation rules:
  - trimmed, non-empty server names
  - `Stdio.command` non-empty
- Pin output capture bounds (65,536 bytes each) and truncation marker `…(truncated)` (UTF-8 preserved).

### Out

- Any normalization of stdout/stderr into a universal structured schema (v1 returns bounded text output).
- Any universalization of backend-specific MCP extras.

## Primary interfaces (contracts)

### Inputs

- `AgentWrapperMcpListRequest { context }`
- `AgentWrapperMcpGetRequest { name, context }`
- `AgentWrapperMcpAddRequest { name, transport, context }`
- `AgentWrapperMcpRemoveRequest { name, context }`

### Outputs

- `AgentWrapperMcpCommandOutput { status, stdout, stderr, stdout_truncated, stderr_truncated }`
- Errors (fail-closed):
  - `AgentWrapperError::UnknownBackend`
  - `AgentWrapperError::UnsupportedCapability { agent_kind, capability }`
  - `AgentWrapperError::InvalidRequest` (validation)

## Key invariants / rules

- The API is **non-run** and MUST NOT emit stdout/stderr as run events.
- Capability gating is fail-closed; unadvertised operations return `UnsupportedCapability`.
- Output capture budgets and truncation semantics are pinned and deterministic.

## Dependencies

- **Blocks**:
  - SEAM-2/3/4/5 (all other seams depend on the public contract)
- **Blocked by**: none

## Touch surface

- `docs/specs/universal-agent-api/mcp-management-spec.md` (contract source of truth; may need clarifications)
- `crates/agent_api/src/lib.rs` (public exports / module wiring)
- `crates/agent_api/src/bounds.rs` (likely reuse for stdout/stderr truncation)
- `crates/agent_api/src/backends/*` (trait surface + gateway wiring)

## Verification

- Unit tests for request validation (names trimmed/non-empty; command non-empty).
- Unit tests for output truncation markers and `*_truncated` flags.
- Compile-time “non-run boundary” sanity: MCP APIs return data directly and do not use the run event pipeline.

## Risks / unknowns

- None (pinned: Claude rejects `Url.bearer_token_env_var` as `InvalidRequest`; see SEAM-4).

## Rollout / safety

- Non-breaking additive trait evolution (default hooks fail-closed).
- Capability advertising governs reachability; write ops remain disabled by default (SEAM-2).
