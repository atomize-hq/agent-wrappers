# SEAM-3 — Backend termination responsibilities (CA-C03)

## Seam Brief (Restated)

- **Seam ID**: SEAM-3
- **Name**: Backend termination responsibilities (CA-C03)
- **Goal / value**: Ensure built-in backends (Codex + Claude Code) provide best-effort termination of the spawned CLI process when explicit cancellation is requested, without leaking raw backend output and without undermining drain-on-drop posture.
- **Type**: integration (backend adoption of explicit cancellation + termination hooks)
- **Scope**
  - In:
    - Advertise `agent_api.control.cancel.v1` for built-in backends that implement explicit cancellation.
    - Implement `AgentWrapperBackend::run_control(...)` for built-in backends by delegating to the harness control-path entrypoint (SEAM-2).
    - Provide a backend-specific “request termination” hook for cancellation that is best-effort and idempotent.
    - Preserve redaction/safety posture on cancellation:
      - cancellation completion is pinned to `"cancelled"` (CA-C01) and MUST NOT include raw stdout/stderr.
  - Out:
    - The public API contract and pinned cancellation semantics (SEAM-1 / CA-C01).
    - The harness driver semantics and finality/drain-on-drop invariants (SEAM-2 / CA-C02).
    - Harness-level integration tests and process-level behavior tests (SEAM-4).
- **Touch surface**:
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/claude_code.rs`
  - (If needed for termination hooks) wrapper internals in:
    - `crates/codex/src/exec/streaming.rs`
    - `crates/claude_code/src/client/mod.rs`
- **Verification**:
  - Built-in backend capabilities include `agent_api.control.cancel.v1` iff `run_control(...)` is implemented.
  - `run_control(...)` returns a cancel handle that, when invoked, causes best-effort backend termination via the backend hook (observable in SEAM-4 integration tests).
  - Cancellation does not leak raw backend output in error messages or events (redacted only).
- **Threading constraints**
  - Upstream blockers:
    - `CA-C01` (SEAM-1): public cancellation surface + pinned completion error shape/string.
    - `CA-C02` (SEAM-2): harness cancellation driver semantics + control-path entrypoint + termination hook callout.
  - Downstream blocked seams:
    - `SEAM-4` (tests) depends on SEAM-3 adoption and termination semantics existing.
  - Contracts produced (owned):
    - `CA-C03` — Backend termination contract
  - Contracts consumed:
    - `CA-C01` (SEAM-1): capability id + pinned `"cancelled"` semantics.
    - `CA-C02` (SEAM-2): cancellation signal is observed by pump + completion sender; termination hook is invoked on cancellation.

## Slice index

- `S1` → `slice-1-backend-adoption.md`: Adopt explicit cancellation in built-in backends (`run_control(...)` + capability advertisement) without changing harness semantics.
- `S2` → `slice-2-termination-hooks.md`: Implement best-effort termination hooks for Codex + Claude Code processes (idempotent, redaction-safe).

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `CA-C03` (SEAM-3): built-in backend termination behavior on cancellation
    - Lives in:
      - Pack: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md`
      - Code: `crates/agent_api/src/backends/codex.rs`, `crates/agent_api/src/backends/claude_code.rs`
    - Produced by:
      - `S1` (adoption + capability)
      - `S2` (termination hooks)
- **Contracts consumed**:
  - `CA-C01` (SEAM-1): pinned control plane + `"cancelled"` completion outcome
    - Consumed by: `S1` (capability gating + control path) and `S2` (ensure no raw output leaks on termination).
  - `CA-C02` (SEAM-2): harness cancellation driver semantics
    - Consumed by: `S1` (delegation to harness control entrypoint) and `S2` (termination hook invoked by harness on cancel).
- **Dependency edges honored**:
  - `SEAM-1 (contract)` → `SEAM-2 (harness wiring)` → `SEAM-3 (backend hooks)` → `SEAM-4 (tests)`
    - This plan only implements SEAM-3’s backend-side responsibilities and leaves integration tests to SEAM-4.
- **Parallelization notes**:
  - What can proceed now:
    - `S1` and `S2` can be implemented once SEAM-2’s control-path entrypoint is available (or in parallel behind a local stub if SEAM-2 is in-flight).
  - What must wait:
    - SEAM-4 process-level tests validating termination and pinned cancellation outcomes.

