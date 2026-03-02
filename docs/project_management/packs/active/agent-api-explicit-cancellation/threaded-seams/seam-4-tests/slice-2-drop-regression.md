### S2 — Drop receiver regression (drain-on-drop + completion gating; no deadlocks)

- **User/system value**: Prevent regressions where dropping the universal events receiver stalls draining or deadlocks completion (especially under backpressure).
- **Pinned test parameters (v1)**:
  - `FIRST_EVENT_TIMEOUT`: `1s` (prove the fake process started)
  - `DROP_COMPLETION_TIMEOUT`: `3s` (after dropping `events`, `completion` resolves)
  - `MANY_EVENTS_N`: `200` (backpressure/drain regression stimulus)
  - These timeouts are the same on all supported platforms as defined by SEAM-4 (`seam-4-tests.md`).
- **Scope (in/out)**:
  - In:
    - A regression test that drops `events` early (without calling explicit `cancel()`):
      - the harness continues draining backend event streams ([BH-C04](../../seam-2-harness-cancel-propagation.md#bh-c04-drain-on-drop-posture) posture),
      - completion resolves (DR-0012 / [BH-C05](../../seam-2-harness-cancel-propagation.md#bh-c05-completion-gating-consumer-opt-out-dr-0012) opt-out behavior) without deadlocking.
  - Out:
    - Explicit cancellation (covered by `S1`).
- **Acceptance criteria**:
  - Dropping `events` does not deadlock the run.
  - `completion` resolves within `DROP_COMPLETION_TIMEOUT` after dropping `events`.
- **Dependencies**:
  - Existing BH-C04/BH-C05 semantics (defined in the pack’s SEAM-2 doc) in `backend_harness/runtime.rs`
    and `run_handle_gate.rs`.
- **Verification**:
  - `cargo test -p agent_api --features codex` (or whichever backend is used for the fake process).
- **Rollout/safety**:
  - Prefer a fake process scenario that emits many events (to create backpressure potential) before exiting.

#### S2.T1 — Add a “many events then exit” fake process scenario

- **Outcome**: Fake backend binary supports a scenario like `many_events_then_exit` that emits many JSONL events quickly and then exits successfully.
- **Inputs/outputs**:
  - Input/output: `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
- **Implementation notes**:
  - Emit a `thread.started`, then `MANY_EVENTS_N` additional events (`MANY_EVENTS_N == 200`) to exercise draining.
  - Exit with success status.
- **Acceptance criteria**:
  - The process exits deterministically after emitting N events.
- **Test notes**:
  - Validated via `S2.T2`.
- **Risk/rollback notes**:
  - Low risk; test-only binary.

Checklist:
- Implement: add scenario arm + deterministic event generation.
- Validate: ensure flush behavior (stdout lines are visible promptly).

#### S2.T2 — Add integration test: dropping `events` early does not deadlock completion

- **Outcome**: Integration test that:
  - starts a run via `run(...)`,
  - receives one event,
  - drops the `events` stream without calling explicit `cancel()`,
  - awaits `completion` with a timeout and asserts it resolves.
- **Inputs/outputs**:
  - Output: test case in `crates/agent_api/tests/c3_explicit_cancellation.rs` (or a new `c3_drop_regression.rs`).
  - Inputs:
    - fake process scenario `many_events_then_exit`
    - built-in backend (Codex suggested) configured with fake binary.
- **Implementation notes**:
  - Observe at least one event within `FIRST_EVENT_TIMEOUT` (prove the process started), then drop `events`.
  - Use the pinned timeout `DROP_COMPLETION_TIMEOUT` and ensure the fake process actually exits.
  - Assert completion success (or at least “resolved”) and that no panic/deadlock occurs.
- **Acceptance criteria**:
  - Completion resolves within `DROP_COMPLETION_TIMEOUT` after dropping events.
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex --test c3_explicit_cancellation -- --nocapture`
- **Risk/rollback notes**:
  - Medium risk (timing); mitigate via deterministic fake binary behavior and the pinned timeouts.

Checklist:
- Implement: add test + helpers.
- Test: run targeted test.
- Cleanup: keep assertions minimal (“resolves” + no deadlock) to avoid flake.
