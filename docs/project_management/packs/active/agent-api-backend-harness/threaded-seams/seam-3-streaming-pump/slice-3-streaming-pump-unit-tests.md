# S3 — Harness-layer pump tests (fake stream + receiver drop regression)

- **User/system value**: Prevents accidental semantic changes by pinning “drain-on-drop” and “live forwarding” behavior in harness-owned tests, independent of backend adapters.
- **Scope (in/out)**:
  - In:
    - Add deterministic harness-level tests for:
      - receiver drop mid-stream still drains the backend stream fully,
      - at least one event can be forwarded before stream finality (live behavior),
      - finality signaling rule (as defined in S2) is enforced and stable.
    - Use a fake backend stream fixture (no real backends).
  - Out:
    - End-to-end integration tests against real Codex/Claude backends (SEAM-5).
    - `run_handle_gate` gating tests (SEAM-4), except as they consume the pump output later.
- **Acceptance criteria**:
  - Tests live with the harness pump (`crates/agent_api/src/backend_harness.rs` or a sibling internal test module).
  - Tests are deterministic (no flaky timing dependence); use controlled streams and explicit drop points.
  - At least one test fails if the pump stops draining on receiver drop.
  - At least one test fails if the finality signaling rule changes unintentionally.
- **Dependencies**:
  - Slice S2: pinned semantics for drain-on-drop + finality signaling rule.
  - Upstream contract: `BH-C01 backend harness adapter interface` (SEAM-1) sufficient to build a minimal harness entrypoint or toy adapter if needed.
- **Verification**:
  - `cargo test -p agent_api --features codex`
  - `cargo test -p agent_api --features claude_code`
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S3.T1 — Fake stream fixture that can prove “fully drained”

- **Outcome**: A deterministic fake stream that:
  - emits a known sequence of typed events,
  - allows the test to force receiver drop at a specific point, and
  - exposes an “all events consumed” signal so the test can assert full drain.
- **Inputs/outputs**:
  - Output: test fixture in `crates/agent_api/src/backend_harness.rs` (or sibling test module)
- **Implementation notes**:
  - Prefer a stream backed by an internal `mpsc`/vec iterator with explicit yield points rather than timers.
  - Provide a shared counter/flag (e.g., `AtomicUsize`) to assert the stream was fully consumed even after receiver drop.
- **Acceptance criteria**:
  - The fixture can deterministically prove that all events were consumed by the pump.
- **Test notes**: fixture-only task; used by S3.T2/S3.T3.
- **Risk/rollback notes**: none (tests only).

Checklist:
- Implement: fake stream + “consumed count” signal.
- Test: a trivial fixture sanity test (optional).
- Validate: `cargo test -p agent_api --features codex,claude_code`.
- Cleanup: keep fixture small and harness-local.

#### S3.T2 — Regression test: receiver drop mid-stream still drains fully (BH-C04)

- **Outcome**: A test that would fail if draining stops early when the events receiver is dropped.
- **Inputs/outputs**:
  - Output: harness unit test using the S3.T1 fixture
- **Implementation notes**:
  - Setup:
    - bounded channel `Sender/Receiver` for `AgentWrapperEvent`
    - run the pump in a task
    - receive/forward a small number of events, then drop the receiver intentionally
  - Assert:
    - the backend stream is fully consumed (drained) even after receiver drop
- **Acceptance criteria**:
  - Test fails if the pump exits on receiver drop without draining the stream.
- **Test notes**: keep it deterministic; avoid sleeps.
- **Risk/rollback notes**: none (tests only).

Checklist:
- Implement: receiver-drop drain regression test.
- Test: run `cargo test -p agent_api --features codex,claude_code`.
- Validate: ensure no timing flake (repeatable locally).
- Cleanup: assert on structured outcomes, not stringified errors.

#### S3.T3 — Regression test: “live forwarding” + finality signaling rule

- **Outcome**: A test that pins:
  - at least one forwarded event happens before stream finality (“live” behavior), and
  - the pump’s finality signal (sender drop) occurs only at stream end per S2.
- **Inputs/outputs**:
  - Output: harness unit test(s) co-located with the pump
- **Implementation notes**:
  - Include a receiver-drop path and verify the pump continues draining to stream end without dropping the sender early.
  - Assert finality by observing that the receiver does not reach `None` until the backend stream has ended.
- **Acceptance criteria**:
  - Test fails if the sender is dropped before the backend stream ends.
  - Test fails if no events are forwarded on the happy path.
- **Test notes**: keep mapping hook simple; assert on counts rather than exact ordering unless ordering is explicitly part of the invariant.
- **Risk/rollback notes**: if ordering is discovered to be critical, add an explicit ordering test here before SEAM-5 migration.

Checklist:
- Implement: live-forwarding test + finality signal rule test.
- Test: run full harness test set under feature matrix.
- Validate: deterministic assertions.
- Cleanup: document what invariant each test pins.
