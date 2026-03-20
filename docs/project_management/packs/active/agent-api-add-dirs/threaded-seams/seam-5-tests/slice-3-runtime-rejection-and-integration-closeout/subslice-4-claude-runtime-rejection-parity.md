### S3d — Claude runtime-rejection parity tests

- **User/system value**: Claude wrapper behavior stays pinned when add-dir rejection happens after
  a run handle exists, so hosts get the same one-terminal-error, completion-identical, no-leak
  contract across fresh, resume, and fork selector branches.
- **Scope (in/out)**:
  - In:
    - Add Claude parity tests for fresh run, resume selector `"last"` / `"id"`, and fork selector
      `"last"` / `"id"`.
    - Assert a run handle was already returned before the backend rejection.
    - Assert exactly one terminal `AgentWrapperEventKind::Error` event is emitted while the stream
      is open.
    - Assert `completion` returns
      `Err(AgentWrapperError::Backend { message: "add_dirs rejected by runtime" })`.
    - Assert no raw-path/stdout/stderr sentinel leaks into any user-visible event or completion
      error.
    - Introduce a small Claude-local helper only if it keeps selector-branch tests readable.
  - Out:
    - Codex runtime-rejection parity tests.
    - Capability-matrix regeneration or repo-level closeout commands.
- **Acceptance criteria**:
  - Claude fresh, resume `"last"` / `"id"`, and fork `"last"` / `"id"` branches fail
    deterministically if they emit multiple terminal errors, mismatch event/completion messages, or
    leak any sentinel.
  - The safe message stays exactly `add_dirs rejected by runtime` in both terminal events and
    completion errors.
  - Selector-branch-specific coverage remains explicit rather than inferred through shared generic
    tests.
- **Dependencies**:
  - `S3b`
  - `SEAM-4`
  - `AD-C03`
- **Verification**:
  - `cargo test -p agent_api --all-features claude_code`
- **Rollout/safety**:
  - Keep helper extraction bounded to Claude tests unless a shared helper would obviously reduce
    noise without hiding branch-specific assertions. Land before `S3e`.

#### S3d.T1 — Add Claude wrapper parity coverage for post-handle rejections

- **Outcome**: Claude fresh/resume/fork flows have deterministic wrapper-level tests for the
  post-handle runtime rejection contract across every pinned selector branch.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/tests/`

Checklist:
- Implement:
  - add parity tests for fresh, resume `"last"` / `"id"`, and fork `"last"` / `"id"`
  - assert exactly one terminal `Error` event and completion-identical safe messaging
  - add or reuse a small helper for no-leak and terminal-event counting if it stays Claude-local
- Test:
  - run `cargo test -p agent_api --all-features claude_code`
- Validate:
  - confirm the tests prove a handle already existed before the rejection path
  - confirm `ADD_DIR_RAW_PATH_SECRET`, `ADD_DIR_STDOUT_SECRET`, and `ADD_DIR_STDERR_SECRET` never
    appear in `AgentWrapperEvent.message`, `AgentWrapperEvent.text`, or the backend error message
