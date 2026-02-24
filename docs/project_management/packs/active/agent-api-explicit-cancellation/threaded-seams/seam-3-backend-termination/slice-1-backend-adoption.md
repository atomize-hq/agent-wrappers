### S1 — Built-in backend adoption (`run_control(...)` + capability)

- **User/system value**: Codex and Claude Code backends expose explicit cancellation via the stable `agent_api` surface so orchestrators can request termination deterministically when supported.
- **Scope (in/out)**:
  - In:
    - Implement `AgentWrapperBackend::run_control(...)` for:
      - `CodexBackend`
      - `ClaudeCodeBackend`
    - Advertise `agent_api.control.cancel.v1` iff the backend supports explicit cancellation.
    - Delegate runtime wiring (driver semantics, completion selection, forwarding/draining) to the harness control-path entrypoint (SEAM-2).
  - Out:
    - Implementing harness driver semantics (SEAM-2).
    - Harness-level process termination tests (SEAM-4).
- **Acceptance criteria**:
  - Capabilities:
    - Built-in backends include `agent_api.control.cancel.v1` iff they implement `run_control(...)` (fail-closed otherwise).
  - Control path:
    - `run_control(...)` returns `AgentWrapperRunControl` (handle + cancel) and preserves existing `run(...)` behavior.
  - Safety posture:
    - No raw backend stdout/stderr appears in any cancellation-related error/event text (redacted only).
- **Dependencies**:
  - `CA-C01` (SEAM-1): public cancel surface + pinned semantics.
  - `CA-C02` (SEAM-2): harness control-path entrypoint returning `AgentWrapperRunControl`.
- **Verification**:
  - Compile + unit/backend-module tests that assert capability ids and that `run_control` is wired (non-integration).
- **Rollout/safety**:
  - Additive changes only; do not change `run(...)` behavior or existing capability ids.

#### S1.T1 — Codex backend: advertise `agent_api.control.cancel.v1` and implement `run_control(...)`

- **Outcome**: `CodexBackend` supports explicit cancellation via `run_control(...)` and advertises `agent_api.control.cancel.v1`.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/backends/codex.rs`
  - Output: updated `capabilities()` and `run_control(...)` implementation delegating to harness control entrypoint.
- **Implementation notes**:
  - Keep `run(...)` unchanged (still delegates to `run_harnessed_backend(...)`).
  - Implement `run_control(...)` to delegate to `run_harnessed_backend_control(...)` (or SEAM-2’s chosen name), passing a termination hook to be implemented in `S2`.
- **Acceptance criteria**:
  - Capability id appears only when control path is implemented.
  - `run_control(...)` returns `UnsupportedCapability` if capability is not advertised (defensive).
- **Test notes**:
  - Add/extend backend tests to assert `agent_api.control.cancel.v1` presence and that source references the harness control entrypoint (non-integration).
- **Risk/rollback notes**:
  - Low risk; additive entrypoint.

Checklist:
- Implement: update `capabilities()` and add `run_control(...)` override.
- Test: add/extend backend-module tests.
- Validate: `make check` and `make clippy`.

#### S1.T2 — Claude Code backend: advertise `agent_api.control.cancel.v1` and implement `run_control(...)`

- **Outcome**: `ClaudeCodeBackend` supports explicit cancellation via `run_control(...)` and advertises `agent_api.control.cancel.v1`.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/backends/claude_code.rs`
  - Output: updated `capabilities()` and `run_control(...)` implementation delegating to harness control entrypoint.
- **Implementation notes**:
  - Keep `run(...)` unchanged.
  - Implement `run_control(...)` similarly to Codex, passing a termination hook to be implemented in `S2`.
- **Acceptance criteria**:
  - Capability id appears only when control path is implemented.
- **Test notes**:
  - Add/extend backend tests (non-integration).
- **Risk/rollback notes**:
  - Low risk; additive entrypoint.

Checklist:
- Implement: update `capabilities()` and add `run_control(...)` override.
- Test: add/extend backend-module tests.
- Validate: `make check` and `make clippy`.

#### S1.T3 — Backend-module conformance tests (non-integration)

- **Outcome**: Lightweight tests pin that built-in backends:
  - advertise the capability when implemented, and
  - use the harness control-path entrypoint (without duplicating harness semantics).
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/backends/codex/tests.rs`, `crates/agent_api/src/backends/claude_code/tests.rs`
  - Output: updated tests.
- **Implementation notes**:
  - Prefer “source contains string” tests (consistent with existing patterns) to ensure delegation rather than re-implementing harness logic in backend code.
  - Do not assert runtime termination behavior here; that is SEAM-4.
- **Acceptance criteria**:
  - Tests pass without requiring SEAM-4 fixtures or real processes.
- **Test notes**:
  - Run `cargo test -p agent_api backends::codex::tests` and `cargo test -p agent_api backends::claude_code::tests` (or equivalent module path tests).
- **Risk/rollback notes**:
  - Low risk; tests only.

Checklist:
- Implement: add capability id assertions and delegation assertions.
- Test: run targeted test commands.
- Cleanup: keep tests robust against formatting changes (avoid brittle exact matches).

