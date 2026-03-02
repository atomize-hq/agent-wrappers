# SEAM-4 — Tests

This seam pins the test coverage required to prevent cancellation drift.

## Required tests (v1)

- Harness-level integration test using a fake backend process that blocks until killed:
  - calling `cancel()` causes the fake process to be terminated best-effort (definition / pass-fail):
    - **Necessary**: while the consumer keeps `events` alive, the consumer-visible `events` stream reaches `None`
      within `CANCEL_TERMINATION_TIMEOUT` (this proves the cancel-driven stream-closure rule).
    - **Sufficient**: `completion` resolves within `CANCEL_TERMINATION_TIMEOUT` (this is treated as the termination signal,
      because DR-0012 requires `completion` to be gated on backend process exit).
    - Stream finality alone MUST NOT be treated as a termination signal (explicit cancellation requires closing the stream).
  - completion resolves to `AgentWrapperError::Backend { message: "cancelled" }`
  - no raw backend output leaks into events/errors
  - cancel-handle lifetime is exercised (dropping `events` does not prevent cancellation)
- Regression test: drop events receiver without calling cancel:
  - draining continues and completion gating semantics remain correct (no deadlocks)

## Supported platforms (v1)

The pinned timeouts and pass/fail criteria in this seam are required to hold on:

- CI gate platform: GitHub Actions `ubuntu-latest` (`x86_64-unknown-linux-gnu`).

## Pinned parameters (v1)

SEAM-3’s time bounds and SEAM-4’s pass/fail criteria depend on a small set of **pinned** timing
constants. The canonical source for these parameters and their test-specific pass/fail criteria is the threaded SEAM-4
slice docs:

- `threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`:
  - `FIRST_EVENT_TIMEOUT=1s`
  - `CANCEL_TERMINATION_TIMEOUT=3s`
- `threaded-seams/seam-4-tests/slice-2-drop-regression.md`:
  - `FIRST_EVENT_TIMEOUT=1s`
  - `DROP_COMPLETION_TIMEOUT=3s`
  - `MANY_EVENTS_N=200`
