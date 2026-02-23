# S3 — Harness regression tests: completion does not resolve early

- **User/system value**: Prevents regressions where completion resolves before stream finality (except when the consumer drops the events stream), which would violate DR-0012 finality semantics and create cross-backend drift.
- **Scope (in/out)**:
  - In:
    - Add deterministic tests that cover both paths:
      - **Finality path**: completion does not resolve until the events sender is dropped (stream reaches `None`).
      - **Consumer-drop path**: if the consumer drops the events stream, completion may resolve once the backend completion is ready (while draining continues in the background).
    - Keep tests harness-owned and backend-independent.
  - Out:
    - End-to-end tests against real backends (SEAM-5).
    - Re-validating DR-0012 semantics (this seam only codifies and guards them).
- **Acceptance criteria**:
  - Tests are deterministic (no sleeps required for correctness).
  - At least one test fails if completion resolves early relative to stream finality.
  - At least one test fails if the consumer-drop escape hatch is removed or broken.
- **Dependencies**:
  - S1: the semantic expectations are documented in `run_handle_gate.rs`.
  - S2: the harness canonical builder exists and uses `build_gated_run_handle`.
  - Upstream behavior: SEAM-3 draining rules are honored (these tests should not require real draining, but must not contradict SEAM-3).
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S3.T1 — Add a minimal “gating fixture” for controlled finality and completion

- **Outcome**: A test fixture that can independently control:
  - when the completion oneshot resolves, and
  - when the event sender is dropped (finality), and
  - whether the consumer drops the events stream early.
- **Inputs/outputs**:
  - Output: tests co-located with `crates/agent_api/src/run_handle_gate.rs` or in a small `#[cfg(test)]` module near the harness builder.
- **Implementation notes**:
  - Prefer explicit channels and drop points over timers.
  - Ensure the test can keep the sender alive while resolving completion to prove that gating is actually enforced.
- **Acceptance criteria**:
  - The fixture can make completion “ready” while keeping the events stream non-final, and can assert that `handle.completion` is still pending.
- **Risk/rollback notes**: tests only.

Checklist:
- Implement: gating fixture with explicit drop controls.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Cleanup: keep fixture minimal and private.

#### S3.T2 — Test: completion is gated on stream finality (no consumer drop)

- **Outcome**: A test that fails if completion resolves before the events stream reaches finality.
- **Inputs/outputs**:
  - Output: `#[test]`/`#[tokio::test]` that:
    - builds a run handle via the canonical harness builder (or directly via `build_gated_run_handle` if the harness builder is not yet available),
    - resolves completion early,
    - asserts completion is still pending until the sender is dropped.
- **Implementation notes**:
  - Explicitly hold the sender so the receiver cannot reach `None`.
  - Use `tokio::select!` with a short “should not complete yet” branch to avoid flakiness (no sleeps for correctness).
- **Acceptance criteria**:
  - The test fails if `build_gated_run_handle` stops waiting on finality.
- **Risk/rollback notes**: none.

Checklist:
- Implement: finality-gated completion test.
- Validate: deterministic (repeatable locally).
- Cleanup: assert on readiness/pending, not wall-clock timings.

#### S3.T3 — Test: consumer drop permits completion once backend completion is ready

- **Outcome**: A test that fails if completion is permanently blocked after the consumer drops the events stream.
- **Inputs/outputs**:
  - Output: test that:
    - drops the `events` stream early,
    - resolves completion,
    - asserts the completion future resolves promptly (without requiring sender finality).
- **Implementation notes**:
  - Keep the sender alive during the assertion to prove the escape hatch works (completion should not depend on sender drop once the consumer has dropped the stream).
  - This test does not assert draining behavior; draining is pinned by SEAM-3 tests. It only asserts the gating “consumer drop” behavior.
- **Acceptance criteria**:
  - The test fails if dropping the events stream does not unblock completion gating.
- **Risk/rollback notes**: none.

Checklist:
- Implement: consumer-drop completion unblocks test.
- Validate: deterministic; no sleeps required.
- Cleanup: keep assertions minimal and semantic.

