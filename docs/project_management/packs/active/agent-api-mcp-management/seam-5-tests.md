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

