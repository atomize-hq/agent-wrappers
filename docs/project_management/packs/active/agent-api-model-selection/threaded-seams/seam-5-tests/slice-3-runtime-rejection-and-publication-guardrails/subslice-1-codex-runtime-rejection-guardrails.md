### S3a — Codex runtime rejection guardrails

- **User/system value**: proves a model id that passed pre-spawn validation still fails safely when
  Codex rejects it after the stream opens.
- **Scope (in/out)**:
  - In:
    - the fake Codex scenario `model_runtime_rejection_after_thread_started`
    - Codex runtime/error tests for safe completion failure plus terminal `Error` event parity
    - redaction assertions for raw model ids and fake backend stdout/stderr sentinels
  - Out:
    - Claude runtime-rejection behavior
    - capability-matrix publication checks
- **Acceptance criteria**:
  - The test observes `thread.started` before the rejection surfaces.
  - Completion failure and terminal `AgentWrapperEventKind::Error` expose the same safe message.
  - Exactly one terminal `AgentWrapperEventKind::Error` is emitted.
  - No surfaced message leaks the raw model id or fake stdout/stderr sentinel.
- **Dependencies**:
  - `MS-C04` from `SEAM-1`
  - `MS-C06` from `SEAM-3`
- **Verification**:
  - `cargo test -p agent_api codex`
- **Rollout/safety**:
  - Keep the fake Codex scenario isolated and deterministic so existing Codex fake-flow coverage
    does not drift.

#### S3.T1 — Codex runtime rejection test with terminal Error-event conformance

- **Outcome**: Codex midstream rejection is pinned to one safe backend error and one terminal
  `Error` event.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/app_server.rs`
  - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`
  - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`

Checklist:
- Implement:
  - Add a Codex runtime-rejection test that waits for `thread.started` before asserting failure.
  - Assert the completion error message and final `Error` event message are identical.
  - Assert exactly one terminal `AgentWrapperEventKind::Error`.
  - Extend the fake Codex scenario only as needed to make ordering and redaction deterministic.
- Test:
  - `cargo test -p agent_api codex`
- Validate:
  - Use a unique secret sentinel in fake output and assert it never surfaces.
  - Confirm the test stays scoped to runtime rejection after acceptance, not pre-spawn invalid
    input.
