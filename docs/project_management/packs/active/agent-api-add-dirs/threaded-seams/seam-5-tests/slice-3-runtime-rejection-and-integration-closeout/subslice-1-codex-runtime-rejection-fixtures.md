### S3a — Codex runtime-rejection fake scenarios

- **User/system value**: Codex exec/resume runtime-rejection coverage gets dedicated deterministic
  fixtures, so wrapper parity tests can pin the post-handle `Backend` error contract without
  reusing generic non-success scenarios.
- **Scope (in/out)**:
  - In:
    - Add `add_dirs_runtime_rejection_exec`,
      `add_dirs_runtime_rejection_resume_last`, and `add_dirs_runtime_rejection_resume_id` to the
      fake Codex scenario binary.
    - Emit the safe fixture message `add_dirs rejected by runtime`.
    - Keep `ADD_DIR_RAW_PATH_SECRET`, `ADD_DIR_STDOUT_SECRET`, and `ADD_DIR_STDERR_SECRET`
      backend-private while still making them available for downstream leak assertions.
    - Preserve the pinned pre-failure observable event for each surface (`thread.started` for exec,
      `thread.resumed` for resume branches).
  - Out:
    - Wrapper-level event/completion parity assertions.
    - Claude fixture work.
    - Capability-matrix regeneration or repo-level closeout commands.
- **Acceptance criteria**:
  - The fake Codex binary exposes the three pinned runtime-rejection scenario ids from the seam
    brief.
  - Each scenario emits exactly one expected pre-failure event before the backend rejection.
  - Each scenario uses the exact safe message `add_dirs rejected by runtime`.
  - Sentinel values stay backend-private and are not promoted into wrapper-owned assertions here.
- **Dependencies**:
  - `SEAM-3`
  - `AD-C03`
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-5-tests.md`
- **Verification**:
  - `cargo test -p agent_api --all-features codex`
- **Rollout/safety**:
  - Fixture-only sub-slice. Land before `S3c` so the parity tests can consume stable scenario ids.

#### S3a.T1 — Add dedicated Codex runtime-rejection scenarios

- **Outcome**: Codex exec/resume tests can target dedicated add-dir runtime-rejection fixtures
  instead of generic failure scenarios.
- **Files**:
  - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`

Checklist:
- Implement:
  - add scenario ids for exec, resume selector `"last"`, and resume selector `"id"`
  - emit the safe rejection text `add_dirs rejected by runtime`
  - keep raw-path/stdout/stderr sentinels inside fake-backend payloads or stderr only
- Test:
  - verify each scenario emits the expected pre-failure event before the rejection
  - run `cargo test -p agent_api --all-features codex`
- Validate:
  - confirm the scenario ids match the pinned deterministic fixture matrix exactly
  - confirm none of the new fixtures depend on generic non-success completion behavior
