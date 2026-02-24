### S2 — Best-effort termination hooks (Codex + Claude Code)

- **User/system value**: When cancellation is requested, the backend process is best-effort terminated promptly, reducing resource waste and making cancellation behavior predictable for orchestrators.
- **Scope (in/out)**:
  - In:
    - Provide backend-specific termination behavior invoked by the harness on cancellation (SEAM-2).
    - Ensure termination is idempotent and safe (no panics on repeated calls).
    - Ensure termination preserves redaction/safety posture:
      - no raw stdout/stderr is surfaced due to cancellation.
  - Out:
    - Deciding completion outcome on cancellation (pinned to `"cancelled"` by CA-C01; enforced by SEAM-2).
    - Harness-level tests with fake blocking processes (SEAM-4).
- **Acceptance criteria**:
  - Codex: cancellation triggers best-effort termination of the spawned `codex` CLI process.
  - Claude Code: cancellation triggers best-effort termination of the spawned `claude` CLI process.
  - Termination is idempotent.
  - Termination does not introduce deadlocks or stall draining:
    - pump/drainer continues to drain typed streams until they end (SEAM-2).
- **Dependencies**:
  - `CA-C02` (SEAM-2): harness must invoke a provided termination hook on cancellation.
- **Verification**:
  - Backend-local unit tests can verify that termination hooks trigger the expected wrapper-side cancellation mechanism (without asserting process-level kill success; that is SEAM-4).
- **Rollout/safety**:
  - Prefer internal-only changes to wrapper crates; avoid breaking API surface.

#### S2.T1 — Codex termination hook: ensure cancellation can drop/kill the child process promptly

- **Outcome**: The Codex backend provides a concrete termination hook that, when invoked, best-effort terminates the underlying Codex CLI process.
- **Inputs/outputs**:
  - Inputs:
    - `crates/agent_api/src/backends/codex.rs` (backend wiring)
    - (Optional) `crates/codex/src/exec/streaming.rs` (wrapper cancellation support)
  - Output: a termination hook used by `run_control(...)` that triggers process termination best-effort.
- **Implementation notes**:
  - Codex wrapper uses `kill_on_drop(true)` for the spawned `Command`, but the `Child` is owned by the wrapper’s completion future.
  - One viable strategy:
    - In the agent_api Codex backend control path, spawn the wrapper completion future on a task and keep a `JoinHandle`.
    - The termination hook aborts that task (dropping the future and thus the `Child`), causing best-effort kill via `kill_on_drop(true)`.
  - Ensure stdout/stderr draining tasks do not block indefinitely after cancellation.
- **Acceptance criteria**:
  - Termination hook can be called multiple times without panicking.
  - Cancellation does not cause raw stderr to be surfaced (errors remain redacted).
- **Test notes**:
  - Unit test can assert “termination hook called” behavior via a controllable stub wrapper or by verifying task abortion paths (no real process).
- **Risk/rollback notes**:
  - Medium risk: incorrect handling can lead to stalled completion/draining; keep changes minimal and backed by SEAM-4 integration tests.

Checklist:
- Implement: create/retain a termination handle and wire it into harness control entrypoint.
- Test: add a backend-local unit test around termination hook idempotence (if feasible).
- Validate: `make check` and `make clippy`.

#### S2.T2 — Claude Code termination hook: trigger child kill via wrapper-side cancellation mechanism

- **Outcome**: The Claude Code backend provides a termination hook that causes the spawned CLI process to be killed best-effort.
- **Inputs/outputs**:
  - Inputs:
    - `crates/agent_api/src/backends/claude_code.rs` (backend wiring)
    - `crates/claude_code/src/client/mod.rs` (wrapper implements cancellation on channel close)
  - Output: a termination hook used by `run_control(...)` that triggers wrapper cancellation best-effort.
- **Implementation notes**:
  - `claude_code` stream runner treats receiver closure as cancellation and calls `child.start_kill()`:
    - it observes `events_tx.closed()` and also handles `send` errors.
  - Provide a termination hook that causes the wrapper’s events channel to close (without depending on consumer drop), e.g.:
    - expose a cancel handle from `claude_code` that closes its internal receiver/channel, or
    - structure the agent_api backend control path so it can close/drop the wrapper event receiver on cancellation.
- **Acceptance criteria**:
  - Termination hook is idempotent.
  - Cancellation does not leak raw backend output in errors/events (redaction preserved).
- **Test notes**:
  - Unit test can validate that closing the wrapper channel triggers the runner’s cancellation branch (no real process).
- **Risk/rollback notes**:
  - Medium risk: ensure the harness still drains typed streams to completion (even if the stream ends early after cancellation).

Checklist:
- Implement: add a concrete termination mechanism and wire it into harness control entrypoint.
- Test: add a unit test for termination hook idempotence and cancellation branch activation.
- Validate: `make check` and `make clippy`.

#### S2.T3 — Redaction/safety audit for cancellation paths (Codex + Claude Code)

- **Outcome**: Explicit cancellation + termination does not introduce new surfaces that leak raw backend output.
- **Inputs/outputs**:
  - Inputs:
    - `crates/agent_api/src/backends/codex.rs`
    - `crates/agent_api/src/backends/claude_code.rs`
    - wrapper crates as needed
  - Output: any required redaction adjustments (message content remains bounded/redacted).
- **Implementation notes**:
  - Ensure cancellation completion remains pinned to `"cancelled"` (handled by SEAM-2), and that backend-side termination does not override that with raw error text.
  - Ensure any “non-zero exit” or “timeout” paths continue to redact stderr.
- **Acceptance criteria**:
  - No raw stderr/stdout fragments appear in `AgentWrapperEventKind::Error.message` or `AgentWrapperError::Backend.message` on cancellation path.
- **Test notes**:
  - Lightweight unit tests can assert known strings do not contain raw samples (e.g., “SECRET_*”).
- **Risk/rollback notes**:
  - Low risk; mainly tightening messages.

Checklist:
- Implement: review and adjust redaction mapping where necessary.
- Test: add a regression test using “SECRET_*” sentinel strings if applicable.
- Cleanup: keep messages operator-safe and bounded.

