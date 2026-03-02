# S1 — Extract shared “drain + forward” pump primitive (scaffold `BH-C04`)

- **User/system value**: Creates a single harness-owned orchestration loop that drives backend event draining + forwarding consistently across backends (including drain-on-drop safety), reducing behavioral drift and enabling shared regression tests.
- **Scope (in/out)**:
  - In:
    - Introduce a harness-local pump function (internal) that:
      - consumes a typed backend event stream,
      - forwards mapped events via a bounded `mpsc::Sender<AgentWrapperEvent>` while the receiver is alive.
    - Establish the “shape” required by `BH-C04` without yet fully pinning receiver-drop semantics (that is Slice S2).
  - Out:
    - Backend adoption/migration (SEAM-5).
    - Completion gating wiring (SEAM-4).
    - Backend-specific mapping logic beyond a hook surface.
- **Acceptance criteria**:
  - A harness-local pump function exists in `crates/agent_api/src/backend_harness.rs` and compiles under both `--features codex` and `--features claude_code`.
  - The pump can forward at least one mapped event on a happy path where the receiver stays alive.
  - The pump does **not** own or poll the backend completion future (that ownership belongs to the SEAM-4 completion sender task).
  - All forwarded events are passed through `crate::bounds`.
  - No unbounded buffering is introduced (bounded channel; no unbounded internal collection of events).
- **Dependencies**:
  - Contract from SEAM-1: `BH-C01 backend harness adapter interface` (pins the typed stream + completion + mapping hook shape).
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code` (even if S1’s tests are minimal; full regression comes in S3).
  - Optional smoke test (can live in `#[cfg(test)]`) proving “forward at least one event while stream is alive”.
  - **Rollout/safety**:
  - Keep the pump internal/private until S2+S3 land and semantics are pinned.

## Atomic Tasks

#### S1.T1 — Define the harness-local pump API surface (internal)

- **Outcome**: A single internal function signature (and minimal supporting types) representing “stream + completion + map + bounded sender” as required by `BH-C04`.
- **Inputs/outputs**:
  - Input: typed backend stream + mapping hook + bounded sender
  - Output: `crates/agent_api/src/backend_harness.rs`
- **Implementation notes**:
  - Prefer a small, explicit signature over a generic framework; this is a risk seam and must stay auditable.
  - Mapping hook semantics are pinned by BH-C01 (and MUST be implemented as such by the pump):
    - `BackendHarnessAdapter::map_event(BackendEvent) -> Vec<AgentWrapperEvent>` (0..N)
    - Mapping is infallible: parse failures MUST surface as `BackendError` at the stream boundary.
    - Ordering:
      - Within one backend event, the pump MUST forward events in the order returned by `Vec`.
      - Across backend events, the pump MUST preserve backend stream order.
  - Keep ownership boundaries explicit: mapping stays backend-owned; bounds and forwarding are harness-owned.
- **Acceptance criteria**:
  - The signature is compatible with both Codex and Claude adapters once they adopt the harness (SEAM-5).
  - The pump is usable as the SEAM-3 “drainer” task described in SEAM-4’s canonical handle builder slice (i.e., it owns the event sender and produces the finality signal by dropping it only at stream end).
- **Test notes**: none required for the signature itself; covered indirectly by S1.T2 and S3.
- **Risk/rollback notes**: if the signature is too “wide,” shrink it before it is used by migrated backends.

Checklist:
- Implement: internal pump signature + minimal helper types.
- Test: compile under `--features codex` and `--features claude_code`.
- Validate: `make clippy`.
- Cleanup: keep everything non-`pub` and local to the harness module.

#### S1.T2 — Implement basic “drain + forward” loop (receiver-alive happy path)

- **Outcome**: A harness-owned loop that drains the backend stream and forwards bounded mapped events while the receiver is alive.
- **Inputs/outputs**:
  - Output: pump implementation in `crates/agent_api/src/backend_harness.rs`
- **Implementation notes**:
  - Forward path must apply `crate::bounds` per emitted `AgentWrapperEvent`.
  - Receiver-drop behavior can be “best effort” in this slice (full semantics pinned in S2), but must not introduce cancellation of the backend stream.
- **Acceptance criteria**:
  - With an alive receiver, at least one event is forwarded (bounded) before stream finality (live behavior).
  - The pump terminates when the backend stream ends.
- **Test notes**: a small harness-local smoke test is acceptable; the determinism/regression suite is S3.
- **Risk/rollback notes**: keep behavior minimal and avoid “clever” buffering; correctness beats micro-optimization.

Checklist:
- Implement: drain loop + forwarding path with bounds.
- Test: minimal smoke test (optional) or compile-only until S3 lands.
- Validate: `cargo test -p agent_api --features codex,claude_code`.
- Cleanup: keep pump logic in one place; avoid duplicating existing backend helpers in this seam.

#### S1.T3 — Document bounded channel sizing guidance (minimum viable)

- **Outcome**: A short, local note documenting bounded channel expectations (no unbounded buffering) and how backpressure interacts with forwarding.
- **Inputs/outputs**:
  - Output: doc comment / module note in `crates/agent_api/src/backend_harness.rs`
- **Implementation notes**:
  - Avoid hard-coding policy that SEAM-5 might need to tune per backend; document a default and where it can be configured.
  - Call out how bounded channels interact with receiver drop (forward flag) which will be pinned in S2.
- **Acceptance criteria**:
  - Guidance exists and is discoverable next to the pump implementation.
- **Test notes**: none.
- **Risk/rollback notes**: documentation-only; can be refined during adoption.

Checklist:
- Implement: short bounded-channel guidance note.
- Test: n/a
- Validate: `make clippy` (doc lint if enabled).
- Cleanup: keep guidance crisp and non-normative outside ADR-0013.
