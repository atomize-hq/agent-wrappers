### S2 — Implement the public Rust surface in `crates/agent_api`

- **User/system value**: `agent_api` exposes the new control-plane entrypoint and types so downstream seams can wire cancellation through the harness and built-in backends without public API churn.
- **Scope (in/out)**:
  - In:
    - Add `AgentWrapperCancelHandle` (opaque, cloneable) and `AgentWrapperRunControl` public types.
    - Add `AgentWrapperBackend::run_control(...)` with a fail-closed default implementation.
    - Add `AgentWrapperGateway::run_control(...)` convenience entrypoint.
    - Add minimal core contract tests for `run_control(...)` error behavior (not harness-level cancellation behavior).
  - Out:
    - Actual cancellation propagation through the harness driver tasks (SEAM-2 / CA-C02).
    - Best-effort backend process termination hooks for built-in backends (SEAM-3 / CA-C03).
    - Harness-level/integration tests that pin runtime cancellation behavior (SEAM-4).
- **Acceptance criteria**:
  - `AgentWrapperGateway::run(...)` remains unchanged (no signature change; existing behavior preserved).
  - New public types exist and match the canonical contract:
    - `AgentWrapperRunControl { handle: AgentWrapperRunHandle, cancel: AgentWrapperCancelHandle }`
    - `AgentWrapperCancelHandle::cancel(&self)` is idempotent.
  - Capability gating is fail-closed for `run_control(...)`:
    - If the backend does not advertise `agent_api.control.cancel.v1`, `run_control(...)` returns
      `AgentWrapperError::UnsupportedCapability { agent_kind, capability: "agent_api.control.cancel.v1" }`
      where `agent_kind` is derived from the backend kind (`self.kind().as_str().to_string()`).
  - Adding the surface does not require immediate changes in existing backends (default impl compiles).
- **Dependencies**:
  - Contract pinned by `S1` (exact signatures, pinned strings).
- **Verification**:
  - `make check` (workspace build).
  - `make clippy` (warnings as errors).
  - `cargo test -p agent_api --test c0_core_contract`.
- **Rollout/safety**:
  - Additive surface only; no existing public signatures removed or changed.

#### S2.T1 — Add `AgentWrapperCancelHandle` as an opaque, idempotent cancellation primitive

- **Outcome**: `crates/agent_api` exports `AgentWrapperCancelHandle` with `cancel()` and no public fields, suitable for downstream wiring (SEAM-2/3) without leaking implementation.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/lib.rs`
  - Output: new `AgentWrapperCancelHandle` type + internal representation and constructor(s) as needed.
- **Implementation notes**:
  - Public API:
    - `#[derive(Clone)] pub struct AgentWrapperCancelHandle { /* private */ }`
    - `impl AgentWrapperCancelHandle { pub fn cancel(&self) { ... } }`
  - Provide a crate-private constructor for harness/backends to create it without exposing internals.
  - Enforce idempotence in the handle layer (e.g., `AtomicBool` guard) so downstream code can call
    `cancel()` freely without duplicating idempotence logic.
- **Acceptance criteria**:
  - `cancel()` is safe to call multiple times and does not panic.
  - The type is constructible only within the crate (or by backends) via crate-private API.
- **Test notes**:
  - Unit test can assert idempotence at the handle level without requiring SEAM-2 wiring.
- **Risk/rollback notes**:
  - Low risk (new type). If design friction appears for SEAM-2/3, adjust only the crate-private constructor, not the public type.

Checklist:
- Implement: add type + crate-private constructor + idempotence guard.
- Test: add a small unit test for idempotence (if it fits without SEAM-2 wiring).
- Validate: `make check` and `make clippy`.
- Cleanup: ensure rustdoc matches `docs/specs/universal-agent-api/contract.md`.

#### S2.T2 — Add `AgentWrapperRunControl` public type

- **Outcome**: `crates/agent_api` exports `AgentWrapperRunControl { handle, cancel }` matching the canonical contract.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/lib.rs`
  - Output: new `AgentWrapperRunControl` type.
- **Implementation notes**:
  - Keep this a simple carrier type; do not add behavior beyond struct construction/Debug.
- **Acceptance criteria**:
  - Field names and types match the canonical contract exactly.
- **Test notes**:
  - Compilation is the main verification.
- **Risk/rollback notes**:
  - Low risk; additive type.

Checklist:
- Implement: add type + Debug behavior consistent with other handles.
- Validate: `make check`.
- Cleanup: keep type placement near `AgentWrapperRunHandle` for discoverability.

#### S2.T3 — Extend `AgentWrapperBackend` with a default `run_control(...)` fail-closed implementation

- **Outcome**: Backends compile unchanged, and explicit cancellation is fail-closed by default.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/lib.rs` (`pub trait AgentWrapperBackend`)
  - Output: added `run_control(...)` method with default impl returning `UnsupportedCapability`.
- **Implementation notes**:
  - Default behavior must match CA-C01 exactly:
    - `AgentWrapperError::UnsupportedCapability { agent_kind, capability: "agent_api.control.cancel.v1" }`
      where `agent_kind` is derived from the backend kind (`self.kind().as_str().to_string()`).
- **Acceptance criteria**:
  - Existing backends (including feature-gated built-ins) compile without changes.
  - Backends that want explicit cancellation can override `run_control(...)` in SEAM-2/3.
- **Test notes**:
  - Add a small core test that a dummy backend without the capability fails closed.
- **Risk/rollback notes**:
  - Low risk; additive trait method with default impl.

Checklist:
- Implement: add method with default impl.
- Test: cover default fail-closed behavior via a dummy backend in `c0_core_contract`.
- Validate: `make check` and `make clippy`.
- Cleanup: keep doc comment consistent with `docs/specs/universal-agent-api/contract.md`.

#### S2.T4 — Add `AgentWrapperGateway::run_control(...)` convenience entrypoint

- **Outcome**: Consumers can call `gateway.run_control(&kind, request)` and get an explicit cancellation handle when supported.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/lib.rs` (`impl AgentWrapperGateway`)
  - Output: new `run_control(...)` method.
- **Implementation notes**:
  - Preserve existing `run(...)` behavior.
  - Failures:
    - Unknown backend: `AgentWrapperError::UnknownBackend`.
    - Missing capability: `AgentWrapperError::UnsupportedCapability { agent_kind, capability: "agent_api.control.cancel.v1" }`
      where `agent_kind` is derived from the requested `AgentWrapperKind` (`agent_kind.as_str().to_string()`).
  - Prefer fail-fast capability check in the gateway (based on `backend.capabilities()`), then call
    `backend.run_control(request)`.
- **Acceptance criteria**:
  - Behavior matches CA-C01 for unknown backend and unsupported capability.
  - Signature matches the canonical contract style (`Pin<Box<dyn Future<...>>>`).
- **Test notes**:
  - Core tests should cover unknown backend and missing capability.
- **Risk/rollback notes**:
  - Low risk; additive API surface.

Checklist:
- Implement: add method + capability check.
- Test: add/extend `c0_core_contract` tests.
- Validate: `make check`.
- Cleanup: ensure docs on the method point to `run-protocol-spec.md` for semantics.

#### S2.T5 — Add/extend core contract tests for `run_control(...)` (non-integration)

- **Outcome**: `crates/agent_api/tests/c0_core_contract.rs` covers the new API surface’s basic error contracts without overlapping SEAM-4’s harness-level tests.
- **Inputs/outputs**:
  - Input: `crates/agent_api/tests/c0_core_contract.rs`
  - Output: new tests for `run_control(...)` basic behavior.
- **Implementation notes**:
  - Add tests:
    - `gateway_run_control_unknown_backend_is_error`.
    - `gateway_run_control_missing_capability_is_unsupported` (dummy backend without `agent_api.control.cancel.v1`).
  - Do not test runtime cancellation termination here; that belongs to SEAM-4.
- **Acceptance criteria**:
  - Tests pass without requiring SEAM-2/3 wiring.
- **Test notes**:
  - Run `cargo test -p agent_api --test c0_core_contract`.
- **Risk/rollback notes**:
  - Low risk; tests only.

Checklist:
- Implement: add tests for unknown backend + unsupported capability.
- Test: run targeted test command.
- Validate: ensure no overlap with SEAM-4 integration tests.
- Cleanup: keep test naming aligned with existing file.
