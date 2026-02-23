# S2 — Centralize run-handle construction in the harness using `run_handle_gate`

- **User/system value**: Ensures all harness-driven backends get consistent DR-0012 completion gating by construction, removing per-backend variation risk and making SEAM-5 adoption “can’t forget the gate.”
- **Scope (in/out)**:
  - In:
    - Add/confirm a single harness-owned run-handle builder that:
      - creates the bounded `mpsc` channel for universal events,
      - spawns the harness driver task(s) (SEAM-3 pump + completion extraction),
      - returns `AgentWrapperRunHandle` via `run_handle_gate::build_gated_run_handle`.
    - Ensure the adapter contract shape (`BH-C01`) cannot bypass the canonical gating path.
  - Out:
    - Migrating existing backends to the harness (SEAM-5), except for any minimal wiring needed to compile the harness entrypoint.
    - Redefining the SEAM-3 finality signaling + drain-on-drop semantics (treated as upstream).
- **Acceptance criteria**:
  - There is exactly one harness entrypoint that constructs `AgentWrapperRunHandle`, and it always uses `build_gated_run_handle`.
  - Backend adapters (per `BH-C01`) provide `(typed stream, completion future, mapping)` but do not construct `AgentWrapperRunHandle` directly.
  - Completion cannot resolve “early” relative to stream finality unless the consumer drops the events stream (enforced by S3 tests).
- **Dependencies**:
  - `BH-C01 backend harness adapter interface` (SEAM-1): harness owns run-handle construction.
  - `BH-C04 stream forwarding + drain-on-drop` (SEAM-3): harness driver produces finality and completion channels in the pinned way.
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code` (even if SEAM-5 adoption isn’t complete, the harness module should compile + tests should run).

## Task ownership + cancellation semantics (pinned) (BH-C05)

This slice pins task ownership so SEAM-5 migrations cannot accidentally introduce premature
cancellation or leaked tasks.

### Driver task ownership (pinned)

`backend_harness::run_harnessed_backend(...)` MUST spawn exactly two detached tasks:

1) **Pump/drainer task** (BH-C04 owner)
   - Owns:
     - the typed backend events stream, and
     - the universal `mpsc::Sender<AgentWrapperEvent>` (finality signal).
   - Lifecycle:
     - exits only when the typed backend stream ends (drained to completion),
     - drops the sender only at typed stream end (finality).

2) **Completion sender task**
   - Owns:
     - the typed backend completion future, and
     - the universal completion `oneshot::Sender<Result<AgentWrapperCompletion, AgentWrapperError>>`.
   - Lifecycle:
     - awaits the completion future once,
     - maps to `AgentWrapperCompletion` (or `AgentWrapperError`) via BH-C01 rules,
     - sends the outcome on the oneshot and exits.

JoinHandle policy (pinned):

- Both tasks are spawned via `tokio::spawn(...)`.
- Their `JoinHandle`s MUST be dropped immediately (detached tasks).
- The lifetime of each task is therefore owned by the resources it holds (stream/future/senders),
  not by the `AgentWrapperRunHandle` value.

### Cancellation semantics (pinned; v1)

- Dropping the `AgentWrapperRunHandle` value MUST NOT cancel the pump/drainer task.
  - This preserves BH-C04 “drain-on-drop” and prevents accidental backend cancellation/deadlocks.
- Dropping only the events stream:
  - MUST unblock completion observability via `run_handle_gate` (consumer opt-out),
  - MUST NOT stop backend draining; the pump transitions to `forward=false` and drains to end.
- Dropping the completion future:
  - MAY cause the completion sender’s `oneshot::send(...)` to fail; this is not fatal.
  - The pump continues draining regardless.

Resource cleanup (v1):

- The harness does not implement a generic “terminate backend process” API in v1.
- Best-effort cancellation remains backend-owned and is expressed only via:
  - dropping/ending the underlying typed stream/future when the wrapper runtime supports it, and/or
  - backend-specific wrapper behavior.

### Pinned tests required by this slice

- DR-0012 completion gating is already pinned by:
  - `crates/agent_api/tests/dr0012_completion_gating.rs`
- Task ownership is pinned indirectly by the BH-C04 pump tests (receiver drop does not stop draining)
  and by a harness test that asserts completion can become observable after events stream drop even
  while draining continues in the background.

## Atomic Tasks

#### S2.T1 — Implement a canonical “build gated run handle” path inside the harness

- **Outcome**: A harness function that is the only location where `AgentWrapperRunHandle` is constructed for harness-driven backends.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backend_harness.rs`
  - Output: `crates/agent_api/src/run_handle_gate.rs` (usage only; no semantic change)
- **Implementation notes**:
  - Create:
    - bounded `mpsc::Sender/Receiver<AgentWrapperEvent>`,
    - a `oneshot::Sender/Receiver<Result<AgentWrapperCompletion, AgentWrapperError>>` for the completion outcome,
    - harness driver task(s) with an explicit lifecycle split:
      - **Pump/drainer** (SEAM-3): owns the event `Sender`, forwards while the receiver is alive, and keeps draining the backend stream until end even after receiver drop; it drops the `Sender` only at stream finality.
      - **Completion sender**: awaits the backend completion future and sends the completion outcome on the oneshot as soon as it is ready (independent of draining), so the DR-0012 consumer-drop escape hatch can resolve completion while draining continues in the background.
  - Return `build_gated_run_handle(rx, completion_rx)` as the only `AgentWrapperRunHandle` construction.
  - Ensure the driver task is not dropped early (store join handle if needed, or structure it so the pump + completion outlive the caller).
- **Acceptance criteria**:
  - The harness can return a gated run handle without requiring any backend-local “gate wiring” code.
  - The implementation does not change `AgentWrapperRunHandle` shape or public API.
- **Test notes**: enforced by S3 regression tests.
- **Risk/rollback notes**: if subtle cancellation bugs appear, prefer simplifying task ownership over adding complex synchronization.

Checklist:
- Implement: canonical handle builder in the harness.
- Test: run S3 regression tests (once landed).
- Validate: `make clippy`.
- Cleanup: keep the builder small and auditable.

#### S2.T2 — Enforce “adapters do not build run handles” via the `BH-C01` contract shape

- **Outcome**: The harness contract surface makes it impossible (or at least unnatural) for adapters to bypass gating.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backend_harness.rs` (contract definition from SEAM-1, updated only if needed)
- **Implementation notes**:
  - The adapter’s spawn return type must remain “typed stream + typed completion future” (not `AgentWrapperRunHandle`).
  - Any adapter-provided hooks must be “data providers” (kind, allowlists, mapping, error mapping), not lifecycle controllers.
- **Acceptance criteria**:
  - Reviewers can point to one function (“the harness builder”) and be confident DR-0012 gating is always applied.
- **Test notes**: n/a.
- **Risk/rollback notes**: avoid over-engineering; the contract’s simplicity is a safety property.

Checklist:
- Validate: `BH-C01` does not expose run-handle construction hooks.
- Validate: `BH-C05` is applied only in the harness builder.
- Cleanup: document the intended invariant in `backend_harness.rs`.

#### S2.T3 — Wire the harness driver to produce the correct finality signal for gating

- **Outcome**: The harness driver drops the universal event sender only when stream finality is reached per the SEAM-3 pinned rules, so `run_handle_gate` finality gating is meaningful.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backend_harness.rs` (driver/pump integration)
- **Implementation notes**:
  - Treat “drop sender” as a finality signal: it must happen at the right time to prevent early completion.
  - Ensure that consumer drop does not stop draining (SEAM-3), even though gating will permit completion to resolve after consumer drop.
- **Acceptance criteria**:
  - The finality signal matches the S3 pinned test expectations.
- **Test notes**: exercised by S3 regression tests.
- **Risk/rollback notes**: keep “drop sender” decisions centralized to avoid implicit finality leaks.

Checklist:
- Implement: driver drop points are explicit and documented.
- Test: S3 completion gating regression test(s).
- Validate: no hidden `drop(tx)` scattered across modules.
- Cleanup: keep lifecycle ordering straightforward.
