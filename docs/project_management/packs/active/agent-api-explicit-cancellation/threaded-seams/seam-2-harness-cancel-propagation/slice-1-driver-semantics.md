### S1 — Cancellation driver semantics (CA-C02)

- **User/system value**: Deterministic, non-deadlocking cancellation behavior at the harness driver layer that preserves drain-on-drop and run finality semantics.
- **Scope (in/out)**:
  - In:
    - A shared cancellation signal observed by:
      - pump/drainer (event forwarding + draining), and
      - completion sender (completion resolution).
    - Correct behavior on cancellation:
      - stop forwarding events immediately (no new consumer-visible events),
      - close the universal events stream (consumer sees stream end),
      - keep draining the typed backend stream to completion (BH-C04; see [BH-C04](../../seam-2-harness-cancel-propagation.md#bh-c04-drain-on-drop-posture)),
      - select the pinned cancellation error when cancellation is requested before backend completion,
        but still obey completion gating (DR-0012 / [BH-C05](../../seam-2-harness-cancel-propagation.md#bh-c05-completion-gating-consumer-opt-out-dr-0012)): completion MUST NOT resolve before the
        underlying backend process exits, and MUST wait for consumer-visible stream finality unless
        the consumer opts out by dropping `events`.
    - Keep receiver-drop semantics unchanged (drop is still best-effort cancellation; draining continues).
  - Out:
    - Backend-specific termination mechanics (SEAM-3), beyond invoking a provided “request termination” hook.
- **Acceptance criteria**:
  - Cancellation does not depend on receiver drop and does not depend on timeout wrappers.
  - Pump/drainer:
    - stops forwarding after cancellation,
    - still drains the typed backend stream to completion.
  - Completion sender:
    - if cancellation is requested before `completion` resolves (i.e., before it would resolve as `Ok(...)` or `Err(...)`),
      `completion` resolves to `Err(AgentWrapperError::Backend { message: "cancelled" })` (this MUST override any backend
      error completion that would otherwise occur after cancellation is requested),
    - if backend completion resolves before cancellation is requested, cancellation does not change the already-resolved outcome,
    - tie-breaking (concurrent readiness): cancellation wins (the pinned `"cancelled"` error).
  - No late consumer-visible events after cancellation completion is observed.
  - Cancellation changes the completion *value*, not the completion *timing*:
    - completion MUST still obey DR-0012 completion gating (wait for backend process exit; and wait
      for stream finality unless the consumer opts out by dropping `events`).
- **Dependencies**:
  - `CA-C01` (SEAM-1): pinned `"cancelled"` error string; cancel handle wiring.
  - `CA-C03` (SEAM-3): backend termination hook implementation (invoked here when available).
- **Verification**:
  - Harness-local unit tests (no external processes) that assert cancellation race + draining behavior.
- **Rollout/safety**:
  - Keep existing `run_harnessed_backend(...)` path unchanged; cancellation semantics apply only to the control-path entrypoint added in `S2`.

#### S1.T1 — Define a harness-internal cancellation signal (shared by pump + completion sender)

- **Outcome**: A small, explicit cancellation primitive that can be cloned into both tasks and queried/awaited without spurious wakeups.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/backend_harness/runtime.rs`
  - Output: module-private cancellation signal type (e.g., `Notify` + atomic state) and wiring points.
- **Implementation notes**:
  - Cancellation must be idempotent.
  - The signal must support:
    - “fire once” semantics, and
    - “late subscribers” (tasks created after cancellation should observe it).
- **Acceptance criteria**:
  - Multiple `cancel()` calls do not cause panics and do not create multiple completion sends.
- **Test notes**:
  - Unit test can cancel before spawning the pump and ensure tasks still observe cancellation.
- **Risk/rollback notes**:
  - Low risk; internal-only.

Checklist:
- Implement: add cancellation signal + `is_cancelled()` + `cancel()` + `cancelled().await`.
- Test: add a minimal unit test for idempotence/late subscriber behavior.
- Validate: `make check`.

#### S1.T2 — Teach the pump/drainer to stop forwarding on cancellation while still draining

- **Outcome**: The pump/drainer observes cancellation and:
  - closes the universal event stream (drop `Sender`), and
  - continues draining the typed backend stream to completion without mapping/bounding/sending.
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/backend_harness/runtime.rs` (`pump_backend_events(...)`)
  - Output: cancellation-aware pump/drainer logic.
- **Implementation notes**:
  - Preserve BH-C04 semantics for receiver-drop (see [BH-C04](../../seam-2-harness-cancel-propagation.md#bh-c04-drain-on-drop-posture)):
    - receiver drop still stops forwarding and keeps draining.
  - On explicit cancellation (distinct from receiver-drop), proactively close the universal stream:
    - drop the `mpsc::Sender` (or equivalent) so the consumer sees `None`.
- **Acceptance criteria**:
  - After cancellation, the consumer sees no further events and eventually sees stream termination.
  - Typed backend stream is fully drained even after cancellation.
- **Test notes**:
  - Use a typed stream that counts polls/items to prove it was drained.
- **Risk/rollback notes**:
  - Medium risk (driver behavior); keep changes localized and test-covered.

Checklist:
- Implement: add cancellation observation and “close stream but keep draining” behavior.
- Test: assert no events after cancel; assert drain reached end.
- Validate: `make clippy`.

#### S1.T3 — Teach the completion sender to race backend completion vs cancellation

- **Outcome**: Completion sender resolves completion to:
  - backend completion when it wins, or
  - pinned cancellation error when cancellation is requested before backend completion (value
    selection), while still obeying DR-0012 completion gating (timing).
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/backend_harness/runtime.rs` (completion sender task)
  - Output: select/race logic with pinned error and no outcome override after resolution.
- **Implementation notes**:
  - Cancellation outcome must be exactly:
    - `Err(AgentWrapperError::Backend { message })` where `message == "cancelled"`.
  - If backend completion resolves first, cancellation must not change the already-sent completion.
  - When cancellation wins, invoke a “request termination” hook if available (implemented by SEAM-3).
- **Acceptance criteria**:
  - Only one completion is observable.
  - Cancel-after-completion does not change the completion outcome.
- **Test notes**:
  - Use a completion future that blocks until externally released to test cancel-wins path.
- **Risk/rollback notes**:
  - Medium risk (completion semantics); keep pinned strings centralized.

Checklist:
- Implement: race logic + pinned cancellation error.
- Test: cover cancel-wins and completion-wins.
- Validate: run targeted harness unit tests.

#### S1.T4 — Add harness-local unit tests for CA-C02 driver semantics (non-integration)

- **Outcome**: Tests exist that pin the driver invariants without requiring real child processes (those are SEAM-4).
- **Inputs/outputs**:
  - Input: `crates/agent_api/src/backend_harness/runtime/tests.rs` (or module tests)
  - Output: new unit tests for cancellation semantics.
- **Implementation notes**:
  - Reuse `backend_harness::test_support` patterns (ToyAdapter) where possible.
  - Do not test backend kill mechanics (SEAM-3) or fake blocking processes (SEAM-4).
- **Acceptance criteria**:
  - Tests verify:
    - cancellation closes the universal events stream,
    - completion resolves to `"cancelled"` when cancellation wins,
    - typed stream draining continues after cancellation and after receiver drop.
- **Test notes**:
  - Run `cargo test -p agent_api backend_harness::runtime::tests -- --nocapture` (or equivalent).
- **Risk/rollback notes**:
  - Low risk; tests only.

Checklist:
- Implement: add tests for cancel-wins and completion-wins.
- Test: run targeted test command(s).
- Cleanup: keep assertions pinned to exact strings/semantics from CA-C01.
