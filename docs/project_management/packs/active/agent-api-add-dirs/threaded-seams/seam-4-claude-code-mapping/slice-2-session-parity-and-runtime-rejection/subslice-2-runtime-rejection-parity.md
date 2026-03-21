### S2b — Runtime rejection parity for accepted add dirs

- **User/system value**: if Claude accepts add dirs, returns a handle, and later rejects them at
  runtime, the backend fails in one safe, observable, branch-consistent way.
- **Scope (in/out)**:
  - In:
    - Add-dir-specific runtime rejection classification after handle creation.
    - Terminal `Error` event emission and completion-message parity.
    - Backend-local assertions for one terminal error event and matching completion text.
  - Out:
    - Selector-branch ordering work.
    - Shared fake-runtime scenario ids and exhaustive fixture inventory owned by SEAM-5.
    - Generic unsupported-key or selection-miss handling from earlier seams.
- **Acceptance criteria**:
  - Handle-returning Claude surfaces produce exactly one terminal `AgentWrapperEventKind::Error`
    event for add-dir runtime rejection.
  - Completion surfaces the same safe/redacted backend message as the terminal error event.
  - Generic non-zero exit handling remains reserved for unrelated failures.
- **Dependencies**:
  - Blocked by: `S1`, `AD-C03`, `AD-C04`
  - Unblocks: `S2c`, SEAM-5 runtime-rejection fixture derivation
- **Verification**:
  - Backend-local event/completion parity assertions stay green.
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Keep selection-failure translation separate; add-dir runtime rejection is not a selector miss.
  - Use the backend-owned safe message `add_dirs rejected by runtime`; do not surface raw
    stdout/stderr.

#### S2.T2 — Implement safe runtime rejection parity for accepted add-dir inputs

- **Outcome**: when Claude accepts add dirs, returns a handle, and later rejects them at runtime,
  the backend emits exactly one terminal safe error event and completes with the same safe/redacted
  backend message.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/mapping.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`

Checklist:
- Implement:
  - Add add-dir-specific runtime rejection classification and terminal error propagation.
  - Reuse the terminal-error event path so the event stream closes after one `Error`.
- Test:
  - Add local parity assertions for error-event/completion message matching.
- Validate:
  - Confirm only one terminal `Error` event is emitted before stream close.
  - Confirm unrelated failures still use the generic redaction path.
