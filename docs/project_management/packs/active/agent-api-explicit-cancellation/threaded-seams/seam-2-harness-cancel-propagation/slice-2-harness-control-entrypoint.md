### S2 — Harness control-path entrypoint for explicit cancellation

- **User/system value**: Backends can implement `run_control(...)` by delegating to a harness entrypoint that already encodes CA-C02 semantics, keeping cancellation behavior consistent across built-in backends.
- **Scope (in/out)**:
  - In:
    - Add a harness entrypoint that returns `AgentWrapperRunControl`:
      - `handle`: the same gated run handle semantics as `run_harnessed_backend(...)`.
      - `cancel`: a cancellation handle wired to the harness cancellation signal and (optionally) a backend termination request hook.
    - Keep the existing `run_harnessed_backend(...) -> AgentWrapperRunHandle` unchanged for non-control runs.
  - Out:
    - Updating built-in backends to use this entrypoint (SEAM-3).
    - Advertising `agent_api.control.cancel.v1` from built-in backends (SEAM-3).
- **Acceptance criteria**:
  - The new entrypoint is additive and does not change current `run(...)` behavior.
  - The returned `AgentWrapperCancelHandle::cancel()` triggers CA-C02 semantics implemented in `S1`.
  - The entrypoint supports an optional backend-provided “request termination” hook (implemented by SEAM-3).
- **Dependencies**:
  - `CA-C01` (SEAM-1): `AgentWrapperRunControl` + `AgentWrapperCancelHandle` types and any crate-private constructors for wiring.
  - `S1`: driver semantics.
- **Verification**:
  - Compile + unit tests from `S1` that exercise the control entrypoint.
- **Rollout/safety**:
  - The control entrypoint can exist unused until SEAM-3 adopts it.

#### S2.T1 — Add `run_harnessed_backend_control(...) -> AgentWrapperRunControl` (crate-private)

- **Outcome**: A harness function that returns both the run handle and the cancel handle, encoding CA-C02 semantics.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/backend_harness/runtime.rs` (and re-exports from `mod.rs` as needed)
  - Output: new harness entrypoint returning `AgentWrapperRunControl`.
- **Implementation notes**:
  - The function should:
    1) normalize request,
    2) spawn backend,
    3) spawn cancellation-aware pump and completion tasks (S1),
    4) return `{ handle: gated_handle, cancel: cancel_handle }`.
  - Provide an optional `request_termination: Option<...>` hook parameter:
    - invoked on cancellation best-effort (owned by SEAM-3 to implement).
- **Acceptance criteria**:
  - The entrypoint compiles and can be used by built-in backends without further harness changes.
- **Test notes**:
  - Unit tests can call this entrypoint directly with a ToyAdapter.
- **Risk/rollback notes**:
  - Medium risk (new entrypoint); keep it crate-private and small.

Checklist:
- Implement: add the new entrypoint and any small helper functions/types.
- Test: ensure `S1` tests cover exercising the control-path cancel handle.
- Validate: `make check` and `make clippy`.

#### S2.T2 — Document/encode “no cross-seam adoption” guidance for SEAM-3

- **Outcome**: Clear guidance for SEAM-3 implementers about how to adopt the control entrypoint without changing CA-C02 semantics.
- **Inputs/outputs**:
  - Input: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md`
  - Output: (No file changes in this seam) add a brief note in this slice describing the adoption contract.
- **Implementation notes**:
  - SEAM-3 should:
    - override `run_control(...)` in built-in backends,
    - advertise `agent_api.control.cancel.v1`,
    - provide a termination hook that the harness invokes on cancellation.
- **Acceptance criteria**:
  - SEAM-3 can implement without needing further SEAM-2 changes.
- **Test notes**:
  - Verified later by SEAM-4 integration tests.
- **Risk/rollback notes**:
  - Low risk; informational only.

Checklist:
- Validate: ensure this slice mentions exactly what SEAM-3 must supply (termination hook + capability id).

