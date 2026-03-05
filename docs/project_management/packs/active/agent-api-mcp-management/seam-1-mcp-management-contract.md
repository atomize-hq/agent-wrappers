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
- Implement request validation rules (pinned in `docs/specs/universal-agent-api/mcp-management-spec.md`):
  - Server names: trimmed; empty is `AgentWrapperError::InvalidRequest`.
  - Transport field validation (MUST occur before spawning any backend process; violations are `InvalidRequest` with a
    safe/redacted message that does not echo raw user-provided values):
    - `Stdio`: `command` non-empty; every item in `command` and `args` trimmed + non-empty; `argv = command + args`.
    - `Url`: `url` trimmed + non-empty; parse absolute `http`/`https` URL; `bearer_token_env_var` (if `Some`) trimmed +
      non-empty and matches `^[A-Za-z_][A-Za-z0-9_]*$`.
- Provide a shared validation helper (owned by SEAM-1) that gateway entrypoints and backend hook implementations (SEAM-3/4)
  invoke before any spawn, so invalid requests fail fast and deterministically.
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
  - `AgentWrapperError::Backend` (spawn/timeout/IO faults; non-zero exit status is **not** an error — it is returned in
    `AgentWrapperMcpCommandOutput.status`)

## Key invariants / rules

- The API is **non-run** and MUST NOT emit stdout/stderr as run events.
- Gateway entrypoints MUST preserve deterministic error ordering: resolve backend → capability check → invoke hook
  (UnknownBackend must take precedence over UnsupportedCapability).
- Capability gating is fail-closed; unadvertised operations return `UnsupportedCapability`.
- Output capture budgets and truncation semantics are pinned and deterministic.
- For `InvalidRequest` and `Backend` errors, messages MUST be safe/redacted and MUST NOT include partial stdout/stderr.
  `AgentWrapperMcpCommandOutput` is returned only in the `Ok(...)` case (even if exit status is non-zero).

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

- Unit tests for full request validation (names; `Stdio` argv + trimming; `Url` parsing/scheme; `bearer_token_env_var`
  regex) including safe/redacted `InvalidRequest` messages.
- Unit test pinning gateway error ordering: UnknownBackend must take precedence over UnsupportedCapability.
- Unit test pinning “validate before spawn”: invalid requests MUST NOT spawn/invoke any backend process.
- Unit tests for output truncation markers and `*_truncated` flags.
- Deterministic tests for `Err(Backend)` paths asserting no partial stdout/stderr is surfaced and no output object is
  returned (e.g., timeout-based harness using fake binaries).
- Compile-time “non-run boundary” sanity: MCP APIs return data directly and do not use the run event pipeline.

## Risks / unknowns

- None (pinned: Claude rejects `Url.bearer_token_env_var` as `InvalidRequest`; see SEAM-4).

## Rollout / safety

- Non-breaking additive trait evolution (default hooks fail-closed).
- Capability advertising governs reachability; write ops remain disabled by default (SEAM-2).
