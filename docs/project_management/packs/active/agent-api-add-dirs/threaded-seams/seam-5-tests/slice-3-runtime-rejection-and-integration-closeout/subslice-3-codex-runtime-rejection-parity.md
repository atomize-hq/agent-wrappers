### S3c — Codex runtime-rejection parity tests

- **User/system value**: Codex wrapper behavior stays pinned when add-dir rejection happens after a
  run handle exists, so hosts see one deterministic terminal error event and one completion message
  with no secret leakage.
- **Scope (in/out)**:
  - In:
    - Add Codex parity tests for exec, resume selector `"last"`, and resume selector `"id"`.
    - Assert a run handle was already returned before the backend rejection.
    - Assert exactly one terminal `AgentWrapperEventKind::Error` event is emitted while the stream
      is open.
    - Assert `completion` returns
      `Err(AgentWrapperError::Backend { message: "add_dirs rejected by runtime" })`.
    - Assert no raw-path/stdout/stderr sentinel leaks into any user-visible event or completion
      error.
    - Introduce a small Codex-local helper only if it keeps the per-surface assertions readable.
  - Out:
    - Codex fork coverage, which stays owned by `S2`.
    - Claude runtime-rejection parity tests.
    - Capability-matrix regeneration or repo-level closeout commands.
- **Acceptance criteria**:
  - Codex exec and both resume selector branches fail deterministically if they emit multiple
    terminal errors, mismatch event/completion messages, or leak any sentinel.
  - Codex fork remains excluded because its pinned contract rejects before a handle exists.
  - The safe message stays exactly `add_dirs rejected by runtime` in both terminal events and
    completion errors.
- **Dependencies**:
  - `S3a`
  - `SEAM-3`
  - `AD-C03`
- **Verification**:
  - `cargo test -p agent_api --all-features codex`
- **Rollout/safety**:
  - Keep the helper bounded to Codex tests unless both backends can share it without coupling.
    Land before `S3e`.

#### S3c.T1 — Add Codex wrapper parity coverage for post-handle rejections

- **Outcome**: Codex exec/resume flows have deterministic wrapper-level tests for the post-handle
  runtime rejection contract.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/`

Checklist:
- Implement:
  - add parity tests for exec, resume selector `"last"`, and resume selector `"id"`
  - assert exactly one terminal `Error` event and completion-identical safe messaging
  - add or reuse a small helper for no-leak and terminal-event counting if it stays Codex-local
- Test:
  - run `cargo test -p agent_api --all-features codex`
- Validate:
  - confirm the tests prove a handle already existed before the rejection path
  - confirm `ADD_DIR_RAW_PATH_SECRET`, `ADD_DIR_STDOUT_SECRET`, and `ADD_DIR_STDERR_SECRET` never
    appear in `AgentWrapperEvent.message`, `AgentWrapperEvent.text`, or the backend error message
