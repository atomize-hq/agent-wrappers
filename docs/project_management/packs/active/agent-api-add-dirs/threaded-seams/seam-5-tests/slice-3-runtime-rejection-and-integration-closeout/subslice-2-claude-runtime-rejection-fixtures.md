### S3b — Claude runtime-rejection fake scenarios

- **User/system value**: Claude fresh/resume/fork runtime-rejection coverage gets dedicated
  deterministic fixtures, so wrapper parity tests can pin safe post-handle failures across every
  selector branch without overloading existing generic error scenarios.
- **Scope (in/out)**:
  - In:
    - Add `add_dirs_runtime_rejection_fresh`,
      `add_dirs_runtime_rejection_resume_last`, `add_dirs_runtime_rejection_resume_id`,
      `add_dirs_runtime_rejection_fork_last`, and `add_dirs_runtime_rejection_fork_id` to the fake
      Claude streaming binary.
    - Emit the safe fixture message `add_dirs rejected by runtime`.
    - Keep `ADD_DIR_RAW_PATH_SECRET`, `ADD_DIR_STDOUT_SECRET`, and `ADD_DIR_STDERR_SECRET`
      backend-private while preserving them for downstream no-leak assertions.
    - Preserve the pinned pre-failure observable event for each branch (the first `system_init`
      fixture line).
  - Out:
    - Wrapper-level event/completion parity assertions.
    - Codex fixture work.
    - Capability-matrix regeneration or repo-level closeout commands.
- **Acceptance criteria**:
  - The fake Claude binary exposes all five pinned runtime-rejection scenario ids from the seam
    brief.
  - Each scenario emits the first `system_init` fixture line before the backend rejection.
  - Each scenario uses the exact safe message `add_dirs rejected by runtime`.
  - Sentinel values stay backend-private and are not surfaced through wrapper-owned assertions here.
- **Dependencies**:
  - `SEAM-4`
  - `AD-C03`
  - `docs/project_management/packs/active/agent-api-add-dirs/seam-5-tests.md`
- **Verification**:
  - `cargo test -p agent_api --all-features claude_code`
- **Rollout/safety**:
  - Fixture-only sub-slice. Land before `S3d` so Claude parity tests can consume stable scenario
    ids and fixed selector-branch behavior.

#### S3b.T1 — Add dedicated Claude runtime-rejection scenarios

- **Outcome**: Claude fresh/resume/fork tests can target dedicated add-dir runtime-rejection
  fixtures instead of generic JSON streaming error paths.
- **Files**:
  - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`

Checklist:
- Implement:
  - add scenario ids for fresh, resume `"last"` / `"id"`, and fork `"last"` / `"id"`
  - emit the safe rejection text `add_dirs rejected by runtime`
  - keep raw-path/stdout/stderr sentinels inside fake-backend payloads or stderr only
- Test:
  - verify each scenario emits the first `system_init` fixture line before the rejection
  - run `cargo test -p agent_api --all-features claude_code`
- Validate:
  - confirm the scenario ids match the pinned deterministic fixture matrix exactly
  - confirm none of the new fixtures reuse the existing generic Claude error scenarios
