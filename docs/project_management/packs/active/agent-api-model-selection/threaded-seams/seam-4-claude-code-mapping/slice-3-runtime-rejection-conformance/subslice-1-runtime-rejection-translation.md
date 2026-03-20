### S3a — Runtime rejection translation after stream open

- **User/system value**: Claude model rejection that arrives after `system init` lands on one safe,
  reviewable backend-error path instead of leaking transport details or diverging between
  completion and event-stream surfaces.
- **Scope (in/out)**:
  - In:
    - Narrow runtime model-rejection detection after the Claude stream is already open.
    - Safe `AgentWrapperError::Backend { message }` translation with one terminal
      `AgentWrapperEventKind::Error` event before stream closure.
    - Dedicated fake-Claude scenario plumbing for `model_runtime_rejection_after_init` and local
      parity assertions for message matching and redaction.
  - Out:
    - Canonical spec publication.
    - Root-flag ordering or `--fallback-model` regression coverage from `S2`.
    - Broad cross-backend runtime rejection matrices owned by SEAM-5.
- **Acceptance criteria**:
  - Runtime rejection after `system init` maps to `AgentWrapperError::Backend { message }`.
  - Already-open streams emit exactly one terminal `AgentWrapperEventKind::Error` event before
    closure.
  - Completion and the terminal error event use the same safe/redacted message.
  - Resume/fork selector misses (`no session found` / `session not found`) stay on their existing
    selection-failure paths.
- **Dependencies**:
  - `MS-C04`
  - `S1`
  - `S2`
- **Verification**:
  - Focused `cargo test -p agent_api claude_code` coverage for Claude runtime parity.
  - Review the fake-Claude scenario path to confirm `system init` precedes the terminal failure.
- **Rollout/safety**:
  - Keep detection scoped to explicit model-rejection signals; do not broaden generic non-zero exit
    handling in the same session.
  - Reuse existing safe-redaction helpers instead of introducing a second backend-error formatter.

#### S3.T1 — Implement safe runtime-rejection translation after `system init`

- **Outcome**: Claude runtime rejection after the stream opens emits one safe terminal error event
  and completes with the same backend-owned message.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/mapping.rs`
  - `crates/agent_api/src/backend_harness/runtime.rs`
  - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs` or the closest
    Claude-runtime-focused test module

Checklist:
- Implement:
  - add or finalize the dedicated `model_runtime_rejection_after_init` fake-Claude scenario
  - classify only the model-rejection runtime signals after stream-open
  - route the failure through one shared safe backend-error/event-tail path
- Test:
  - assert `system init`, one terminal `Error` event, and matching completion text
  - assert redaction rather than raw model id/stdout/stderr passthrough
- Validate:
  - confirm selector-miss behavior remains distinct from runtime model rejection
  - confirm unrelated Claude failures still use the generic redaction path
