### S3b — Claude runtime rejection guardrails

- **User/system value**: proves a model id that passed pre-spawn validation still fails safely when
  Claude rejects it after the stream is live.
- **Scope (in/out)**:
  - In:
    - the fake Claude scenario `model_runtime_rejection_after_init`
    - Claude runtime/error tests for safe completion failure plus terminal `Error` event parity
    - redaction assertions for raw model ids and fake backend stdout/stderr sentinels
  - Out:
    - Codex runtime-rejection behavior
    - `--fallback-model` mapping or ordering assertions
    - capability-matrix publication checks
- **Acceptance criteria**:
  - The test observes `system init` before the rejection surfaces.
  - Completion failure and terminal `AgentWrapperEventKind::Error` expose the same safe message.
  - Exactly one terminal `AgentWrapperEventKind::Error` is emitted.
  - No surfaced message leaks the raw model id or fake stdout/stderr sentinel.
- **Dependencies**:
  - `MS-C04` from `SEAM-1`
  - `MS-C07` from `SEAM-4`
- **Verification**:
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Reuse the Codex assertion shape where practical, but keep the fake Claude scenario isolated so
    existing stream-json coverage does not drift.

#### S3.T2 — Claude runtime rejection test with terminal Error-event conformance

- **Outcome**: Claude post-init rejection is pinned to one safe backend error and one terminal
  `Error` event.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`

Checklist:
- Implement:
  - Add a Claude runtime-rejection test that observes `system init` before asserting failure.
  - Assert the completion error message and final `Error` event message are identical.
  - Assert exactly one terminal `AgentWrapperEventKind::Error`.
  - Extend the fake Claude scenario only as needed to make ordering and redaction deterministic.
- Test:
  - `cargo test -p agent_api claude_code`
- Validate:
  - Use a unique secret sentinel in fake output and assert it never surfaces.
  - Keep `--fallback-model` assertions out of this sub-slice.
