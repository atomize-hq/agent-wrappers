# S1 — Public surface + capability-gated gateway/hooks

- **User/system value**: Unblocks downstream seams by freezing the universal MCP management surface (`agent_api::mcp`) and the capability-gated gateway/hooks, without touching any backend mappings yet.
- **Scope (in/out)**:
  - In:
    - Add public `agent_api::mcp` module and pinned type shapes from `docs/specs/universal-agent-api/mcp-management-spec.md`.
    - Add `AgentWrapperGateway::{mcp_list,mcp_get,mcp_add,mcp_remove}` entrypoints with deterministic error ordering:
      - resolve backend → capability check → (validation in S2) → invoke hook.
    - Add default `AgentWrapperBackend::{mcp_list,mcp_get,mcp_add,mcp_remove}` hooks (non-breaking additive evolution; default fail-closed).
    - Define SEAM-1-owned capability id constants for internal use (MM-C01).
  - Out:
    - Request validation details (S2).
    - Output bounds/truncation algorithm helper (S3).
    - Backend enablement/advertising (SEAM-2).
    - Codex/Claude argv mapping + process execution wiring (SEAM-3/4).
- **Acceptance criteria**:
  - `agent_api::mcp` exists and the pinned spec import compiles:
    - `use agent_api::mcp::{AgentWrapperMcpAddRequest, AgentWrapperMcpAddTransport, AgentWrapperMcpCommandContext, AgentWrapperMcpCommandOutput, AgentWrapperMcpGetRequest, AgentWrapperMcpListRequest, AgentWrapperMcpRemoveRequest};`
  - Gateway methods exist with pinned signatures and preserve deterministic ordering:
    - `UnknownBackend` takes precedence over `UnsupportedCapability`.
    - `UnsupportedCapability` is returned without invoking the backend hook when the capability id is not advertised.
  - Backend trait hooks exist and are additive + fail-closed by default.
  - Feature-flag matrix builds cleanly (existing backends compile unchanged).
- **Dependencies**: none.
- **Verification**:
  - `cargo check -p agent_api`
  - `cargo check -p agent_api --features codex`
  - `cargo check -p agent_api --features claude_code`
  - `cargo check -p agent_api --features codex,claude_code`
  - `cargo test -p agent_api --features codex,claude_code` (gateway ordering tests)

## Atomic Tasks

#### S1.T1 — Add `agent_api::mcp` module + pinned type shapes

- **Outcome**: A new public module providing the pinned request/response types and transport enum, using std + serde-friendly types only.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Pinned type shapes (v1)”)
  - Output: `crates/agent_api/src/mcp.rs` + `pub mod mcp;` export from `crates/agent_api/src/lib.rs`
- **Implementation notes**:
  - Keep the types layout aligned with the spec to minimize drift (derive sets + field names).
  - Do not introduce backend-specific types (`codex::*`, `claude_code::*`) into the public API.
- **Acceptance criteria**:
  - The spec import snippet compiles in a downstream crate.
- **Test notes**:
  - Prefer a compile-only doctest or a minimal `#[test]` that type-checks the imports.
- **Risk/rollback notes**: additive API only.

Checklist:
- Implement: add `mcp.rs` + export.
- Test: `cargo check -p agent_api`.
- Validate: ensure no backend-specific public types leak.
- Cleanup: rustfmt.

#### S1.T2 — Define MCP capability ids (MM-C01) for internal use

- **Outcome**: Capability ids are centralized (single source of truth in code) and match the spec strings exactly:
  - `agent_api.tools.mcp.list.v1`
  - `agent_api.tools.mcp.get.v1`
  - `agent_api.tools.mcp.add.v1`
  - `agent_api.tools.mcp.remove.v1`
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Capability ids (v1, normative)”)
  - Output: `crates/agent_api/src/mcp.rs` (or `crates/agent_api/src/lib.rs`) with `pub(crate)` consts used by gateway + backends
- **Implementation notes**:
  - Keep these `pub(crate)` (not public API) unless/until the canonical spec requires exporting them.
- **Acceptance criteria**:
  - Gateway + backend hook defaults can reference the constants without duplicating string literals.
- **Test notes**: exercised by gateway capability-gating tests.
- **Risk/rollback notes**: internal-only constants.

Checklist:
- Implement: add constants in one module.
- Test: compile under feature matrix.
- Validate: string literals match spec exactly.
- Cleanup: rustfmt.

#### S1.T3 — Add default backend MCP hooks (fail-closed)

- **Outcome**: `AgentWrapperBackend` grows additive default methods:
  - `mcp_list`, `mcp_get`, `mcp_add`, `mcp_remove`
  that return `AgentWrapperError::UnsupportedCapability { agent_kind, capability }` by default.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/lib.rs` (trait definition)
- **Implementation notes**:
  - Mirror the existing `run_control` pattern: defaults are implemented on the trait and produce stable `agent_kind` strings via `self.kind().as_str().to_string()`.
- **Acceptance criteria**:
  - Existing backends compile unchanged (no required impl updates).
- **Test notes**:
  - Covered implicitly by compilation + gateway tests.
- **Risk/rollback notes**: additive trait evolution; low risk.

Checklist:
- Implement: add default methods with boxed futures.
- Test: `cargo check -p agent_api --features codex,claude_code`.
- Validate: ensure default error strings match existing conventions.
- Cleanup: rustfmt.

#### S1.T4 — Add gateway MCP entrypoints (resolve backend + capability gate)

- **Outcome**: `AgentWrapperGateway` provides:
  - `mcp_list`, `mcp_get`, `mcp_add`, `mcp_remove`
  that resolve backend → check advertised capability → invoke backend hook (validation comes in S2).
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/lib.rs` (gateway impl)
- **Implementation notes**:
  - Preserve deterministic ordering: `UnknownBackend` before `UnsupportedCapability`.
  - Do not call backend hooks when the capability id is not advertised.
  - These are non-run operations: do not emit `AgentWrapperEvent`s or touch the run pipeline.
- **Acceptance criteria**:
  - Methods match the pinned signatures in the spec.
- **Test notes**:
  - Add unit tests in S1.T5 for ordering + gating.
- **Risk/rollback notes**: additive gateway methods only.

Checklist:
- Implement: add gateway methods + capability checks.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Validate: verify ordering invariants in tests.
- Cleanup: rustfmt.

#### S1.T5 — Add unit tests for gateway ordering + capability gating

- **Outcome**: Deterministic tests for:
  - `UnknownBackend` precedence, and
  - `UnsupportedCapability` when a backend does not advertise the operation.
- **Inputs/outputs**:
  - Output: `#[cfg(test)]` tests in `crates/agent_api/src/lib.rs` (or `crates/agent_api/src/mcp.rs`)
- **Implementation notes**:
  - Use a minimal test backend:
    - advertises no MCP capabilities, and
    - records whether any MCP hook was invoked (must remain `false` when capability is absent).
- **Acceptance criteria**:
  - Tests fail deterministically if ordering regresses or if a hook is called despite missing capability.
- **Test notes**:
  - Run under `--features codex,claude_code` to ensure backends still compile with the trait evolution.
- **Risk/rollback notes**: tests-only; safe to revise.

Checklist:
- Implement: minimal test backend + tests.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Validate: ensure tests are narrowly scoped to gateway invariants.
- Cleanup: avoid overfitting to backend implementations (keep it generic).

## Notes for downstream seams (non-tasking)

- SEAM-2 consumes the capability ids and the gateway/hook surfaces for advertising + enablement.
- SEAM-3/4 will implement backend hooks and should reuse SEAM-1 validation/output helpers from S2/S3 to avoid semantic drift.
