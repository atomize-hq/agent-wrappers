### S1 — Explicit cancellation integration test (fake blocking process)

- **User/system value**: A single, stable end-to-end test that proves explicit cancellation terminates a blocked backend process best-effort and yields the pinned cancellation completion outcome without leaks.
- **Pinned test parameters (v1)**:
  - `FIRST_EVENT_TIMEOUT`: `1s` (prove the fake process started)
  - `CANCEL_TERMINATION_TIMEOUT`: `3s` (after calling `cancel()`, within this timeout:
    - `completion` resolves to the pinned cancellation error (this is treated as the termination signal, because DR-0012
      gates `completion` on backend process exit), and
    - while the consumer keeps `events` alive, the consumer-visible `events` stream reaches `None` (stream closure rule))
  - These timeouts are the same on all supported platforms as defined by SEAM-4 (`seam-4-tests.md`).
- **Scope (in/out)**:
  - In:
    - A fake backend process that:
      - emits at least one valid JSONL event (to prove the process started), then
      - blocks until killed, while emitting a “SECRET” sentinel on stderr.
    - An integration test that:
      - starts a run via `run_control(...)`,
      - calls `cancel()` at least twice (idempotence),
      - asserts best-effort termination (within `CANCEL_TERMINATION_TIMEOUT`),
      - asserts completion resolves to `"cancelled"`,
      - asserts no event/error contains the stderr secret sentinel.
  - Out:
    - Detailed platform-specific guarantees about kill signaling; this is best-effort.
- **Acceptance criteria**:
  - Calling `cancel()` causes the fake backend process to be terminated best-effort, observed via:
    - `completion` resolving within `CANCEL_TERMINATION_TIMEOUT` (termination signal due to DR-0012 gating), and
    - while the consumer keeps `events` alive, the consumer-visible `events` stream reaching `None` within
      `CANCEL_TERMINATION_TIMEOUT` (stream closure rule).
  - Completion gating (DR-0012) is preserved under cancellation:
    - the `"cancelled"` completion MUST NOT resolve before the underlying backend process exits
      (this test uses a fake process that blocks until killed).
  - Cancel-handle lifetime / orthogonality (pinned by `run-protocol-spec.md`):
    - After obtaining `run_control(...)`, dropping `events` MUST NOT prevent cancellation; calling
      `cancel()` still triggers best-effort termination and `completion` resolves to the pinned
      cancellation error (this exercises the DR-0012 “consumer opt-out” path).
  - `completion` resolves to `Err(AgentWrapperError::Backend { message: "cancelled" })`.
  - No raw secret sentinel from backend stderr appears in:
    - any `AgentWrapperEvent.message`,
    - any `AgentWrapperEvent.text`,
    - (if present) any serialized `AgentWrapperEvent.data`,
    - the completion error message.
- **Dependencies**:
  - `CA-C01` (SEAM-1): `run_control(...)` + pinned `"cancelled"`.
  - `CA-C02` (SEAM-2): cancel driver semantics.
  - `CA-C03` (SEAM-3): backend termination hook is invoked and is effective best-effort.
- **Verification**:
  - `cargo test -p agent_api --features codex` (and any additional feature combination used by CI).
- **Rollout/safety**:
  - Use the pinned timeouts above (`1s` / `3s`) to avoid flaky CI; prefer deterministic “block until killed” behavior.

#### S1.T1 — Extend the fake Codex exec-stream scenario binary with a blocking scenario

- **Outcome**: `fake_codex_stream_exec_scenarios_agent_api` supports a scenario like `block_until_killed` that:
  - emits one valid JSONL line, then
  - prints `RAW-STDERR-SECRET-CANCEL` to stderr, then
  - blocks (sleep loop) until terminated.
- **Inputs/outputs**:
  - Input/output: `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
- **Implementation notes**:
  - Ensure stdout is flushed before blocking (so the harness sees an early event).
  - Do not exit normally; rely on kill/termination to end the process.
- **Acceptance criteria**:
  - The process remains alive indefinitely without cancellation, and exits within `CANCEL_TERMINATION_TIMEOUT` when terminated.
- **Test notes**:
  - Can be validated indirectly via `S1.T2`.
- **Risk/rollback notes**:
  - Low risk; test-only binary.

Checklist:
- Implement: add `match` arm for `block_until_killed`.
- Validate: ensure stdout flushes and stderr sentinel is written.
- Cleanup: keep scenario name stable and documented in comments.

#### S1.T2 — Add integration test: explicit cancel terminates and yields pinned completion error

- **Outcome**: New integration test file (or extension to an existing one) that:
  - runs the Codex backend against the fake binary `block_until_killed`,
  - cancels, and
  - asserts pinned completion error `"cancelled"` + no leaks.
- **Inputs/outputs**:
  - Output: a new integration test (suggested):
    - `crates/agent_api/tests/c3_explicit_cancellation.rs`
  - Inputs:
    - `AgentWrapperGateway::run_control(...)` (preferred) or `backend.run_control(...)` (acceptable)
    - `CARGO_BIN_EXE_fake_codex_stream_exec_scenarios_agent_api`
- **Implementation notes**:
  - Configure backend with `CodexBackendConfig.binary = Some(fake_codex_binary())`.
  - Set env var `FAKE_CODEX_SCENARIO=block_until_killed`.
  - Observe at least one event within `FIRST_EVENT_TIMEOUT`, then call `cancel()` (to reduce timing flake).
  - Call `cancel()` at least twice (idempotence).
  - Drain the `events` stream to `None` and assert it reaches `None` within `CANCEL_TERMINATION_TIMEOUT`.
  - Await `completion` and assert it resolves within `CANCEL_TERMINATION_TIMEOUT`.
  - Add a second test case for cancel-handle lifetime/orthogonality:
    - obtain `run_control(...)`,
    - drop `events` (exercise DR-0012 consumer opt-out behavior),
    - call `cancel()` and assert `completion` still resolves to `"cancelled"` within `CANCEL_TERMINATION_TIMEOUT`.
  - Leak assertion:
    - assert the sentinel `RAW-STDERR-SECRET-CANCEL` is absent from all event fields.
- **Acceptance criteria**:
  - Completion resolves within `CANCEL_TERMINATION_TIMEOUT` with the pinned cancellation error.
  - No secret leakage.
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex --test c3_explicit_cancellation -- --nocapture`
- **Risk/rollback notes**:
  - Medium risk (timing); mitigate by waiting for the first event before cancelling and using the pinned timeouts.

Checklist:
- Implement: add test + helpers (`any_event_contains`, `drain_to_none`) as needed.
- Test: run the targeted test.
- Validate: confirm pinned error match uses exact `"cancelled"`.
- Cleanup: keep the test narrowly scoped and deterministic.

#### S1.T3 — (Optional) Repeat S1 cancellation integration test for Claude Code fake binary

- **Outcome**: A parallel integration test for the Claude Code backend using `fake_claude_stream_json_agent_api`, if explicit cancellation is implemented for that backend.
- **Inputs/outputs**:
  - Output: additional test case(s) under `crates/agent_api/tests/` gated by `feature = "claude_code"`.
- **Implementation notes**:
  - Use `FAKE_CLAUDE_SCENARIO` to select a long-running scenario (may require adding one).
  - Ensure cancellation does not depend on consumer receiver drop (explicit cancel is orthogonal).
- **Acceptance criteria**:
  - Same as Codex: completion is `"cancelled"` and no leak.
- **Test notes**:
  - Run: `cargo test -p agent_api --features claude_code ...`
- **Risk/rollback notes**:
  - Optional; include only if SEAM-3 implements explicit cancel for Claude Code.

Checklist:
- Implement: add scenario and test only if Claude cancellation is supported.
