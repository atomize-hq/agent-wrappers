# SEAM-5 — Tests

- **Name**: MCP management regression tests
- **Type**: integration (verification)
- **Goal / user value**: Prevent drift in the universal MCP management surface by pinning capability gating, request validation,
  output bounds, safe default advertising, and backend mappings.

## Scope

### In

- Unit tests for:
  - capability gating (`UnsupportedCapability` fail-closed for each operation),
  - request validation (trimmed/non-empty names; non-empty `Stdio.command`),
  - output truncation semantics (`…(truncated)` marker + `*_truncated` flags; UTF-8 preserved),
  - safe default advertising for write ops (SEAM-2).
  - pinned backend-specific mapping decisions:
    - Codex `list/get` always pass `--json` (SEAM-3),
    - Claude rejects `Url.bearer_token_env_var` as `InvalidRequest` (SEAM-4),
    - Claude target-availability gating for `get/add/remove` (win32-x64 only; SEAM-2/4).
- Integration tests (opt-in if needed) that:
  - run `list/get/add/remove` against an isolated home directory,
  - do not require network access.

### Out

- End-to-end tests that require a real networked MCP server.
- Tests that assert a universal structured output schema (v1 returns bounded stdout/stderr).

## Primary interfaces (contracts)

- `AgentWrapperGateway::{mcp_list,mcp_get,mcp_add,mcp_remove}`
- `agent_api::mcp::{AgentWrapperMcp*Request, AgentWrapperMcpCommandOutput}`

## Key invariants / rules

- Tests must verify MCP management outputs are not emitted as run events.
- Tests must verify default advertising posture stays safe (write ops off unless enabled).

## Pinned assertions (v1)

This section is the single place where SEAM-5 pins “what must be true” across seams.

### Advertising + enablement (SEAM-2)

- Default built-in backends (`allow_mcp_write == false`):
  - Codex advertises:
    - `agent_api.tools.mcp.list.v1`
    - `agent_api.tools.mcp.get.v1`
    - and does **not** advertise `agent_api.tools.mcp.{add,remove}.v1`.
  - Claude Code advertises:
    - `agent_api.tools.mcp.list.v1` on all targets supported by the pinned manifest, and
    - `agent_api.tools.mcp.get.v1` on `win32-x64` only,
    - and does **not** advertise `agent_api.tools.mcp.{add,remove}.v1`.
- Opt-in write enablement (`allow_mcp_write == true`):
  - Codex advertises `agent_api.tools.mcp.{add,remove}.v1`.
  - Claude Code advertises `agent_api.tools.mcp.{add,remove}.v1` on `win32-x64` only.

### Codex mapping (SEAM-3)

- `mcp_list` and `mcp_get` always include `--json` in the spawned argv (no cross-backend output parity implied).

### Claude mapping (SEAM-4)

- `Url { bearer_token_env_var: Some(_) }` is rejected as `AgentWrapperError::InvalidRequest` on targets where `mcp add` is
  implemented (`win32-x64` in the pinned manifest).
- On unsupported targets, `mcp add/get/remove` MUST fail-closed with `UnsupportedCapability` because the capabilities are
  not advertised.

### Isolated homes (SEAM-2)

- When `codex_home` / `claude_home` are set on backend config, integration coverage demonstrates that write operations
  (when enabled) localize state mutation to the isolated root and do not touch user state.

## Dependencies

- **Blocked by**:
  - SEAM-1 (contract + surface)
  - SEAM-2 (enablement + isolation)
  - SEAM-3/4 (mapping)

## Touch surface

- `crates/agent_api/src/**` tests (unit + integration)
- Potentially `crates/agent_api/src/backend_harness/**` (if reusing harness patterns for process isolation)

## Verification (definition of done)

- `make test` passes with new unit coverage.
- Optional integration tests are gated so CI/local runs are deterministic (no network required).
- `make preflight` passes once implementation lands.

## Risks / unknowns

- **Binary availability**: true end-to-end MCP tests may require installed `codex`/`claude` binaries; keep those tests opt-in or
  use hermetic fake binaries where possible.

## Rollout / safety

- Treat tests as the primary guardrail preventing accidental promotion of backend-specific behavior into universal v1.
