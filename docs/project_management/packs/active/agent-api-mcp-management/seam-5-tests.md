# SEAM-5 — Tests

- **Name**: MCP management regression tests
- **Type**: integration (verification)
- **Goal / user value**: Prevent drift in the universal MCP management surface by verifying capability gating, request validation,
  output bounds, safe default advertising, and backend mappings.

## Scope

### In

- Unit tests for:
  - pin the requirements in the canonical MCP management spec:
    - `docs/specs/universal-agent-api/mcp-management-spec.md` → transport validation, context precedence, execution semantics,
      output truncation algorithm, built-in backend behavior, and verification policy.
- Integration tests:
  - run `list/get/add/remove` against an isolated home directory,
  - MUST use hermetic fake `codex` / `claude` binaries by default (CI + local) and MUST run under normal `cargo test` /
    `make test` flow (no opt-in),
  - generate fake binaries per test run (e.g., tempdir) and inject them via backend config `binary` (preferred) or PATH, so
    no real upstream binaries are executed,
  - verify argv + environment precedence by having the fake binaries record received argv and selected env keys,
  - verify isolated-home state mutation by having the fake binaries write sentinel files beneath the injected home root,
  - are deterministic and offline by construction (see the pinned verification policy in the MCP management spec).

### Out

- End-to-end tests that require a real networked MCP server.
- Tests that assert a universal structured output schema (v1 returns bounded stdout/stderr).

## Primary interfaces (contracts)

- `AgentWrapperGateway::{mcp_list,mcp_get,mcp_add,mcp_remove}`
- `agent_api::mcp::{AgentWrapperMcp*Request, AgentWrapperMcpCommandOutput}`

## Key invariants / rules

- Tests must verify MCP management outputs are not emitted as run events.
- Tests must verify default advertising posture stays safe (write ops off unless enabled).
- Tests must verify pinned manifest drift handling: runtime mismatch fails as `Err(Backend)` and does not silently mutate
  advertised capabilities.

## Canonical sources (for tests)

- Spec + mapping truth: `docs/specs/universal-agent-api/mcp-management-spec.md`
- Capability gating + error taxonomy: `docs/specs/universal-agent-api/contract.md`

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
- Integration tests follow the canonical verification policy in `docs/specs/universal-agent-api/mcp-management-spec.md`.
- `make preflight` passes once implementation lands.

## Risks / unknowns

- **Live binary availability**: live smoke tests against real installed `codex`/`claude` binaries are optional and MUST be opt-in
  per the MCP management spec verification policy (`#[ignore]` + `AGENT_API_MCP_LIVE=1` + configured binary path). Default
  integration coverage uses hermetic fake binaries and does not require installed upstream CLIs.

## Rollout / safety

- Treat tests as the primary guardrail preventing accidental promotion of backend-specific behavior into universal v1.
