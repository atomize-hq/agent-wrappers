# SEAM-4 — Tests

This seam pins the test coverage required to prevent cancellation drift.

## Required tests (v1)

- Harness-level integration test using a fake backend process that blocks until killed:
  - calling `cancel()` causes the fake process to be terminated best-effort
  - completion resolves to `AgentWrapperError::Backend { message: "cancelled" }`
  - no raw backend output leaks into events/errors
  - cancel-handle lifetime is exercised (dropping `events` does not prevent cancellation)
- Regression test: drop events receiver without calling cancel:
  - draining continues and completion gating semantics remain correct (no deadlocks)

## Pinned parameters (v1)

SEAM-3’s time bounds and SEAM-4’s pass/fail criteria depend on a small set of **pinned** timing
constants. The canonical source for these parameters is the threaded SEAM-4 slice docs:

- `threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`:
  - `FIRST_EVENT_TIMEOUT=1s`
  - `CANCEL_TERMINATION_TIMEOUT=3s`
- `threaded-seams/seam-4-tests/slice-2-drop-regression.md`:
  - `FIRST_EVENT_TIMEOUT=1s`
  - `DROP_COMPLETION_TIMEOUT=3s`
  - `MANY_EVENTS_N=200`
