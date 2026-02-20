# Kickoff Prompt — C2-code (Validation hardening: fixtures + safety)

## Scope
- Implement C2 production code only (no tests) per:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/C2-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/platform-parity-spec.md`

## Start Checklist
1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `decision_register.md`, `C2-spec.md`, and this prompt.
3. Set `C2-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C2-code`).
5. Create the task branch and worktree: `git worktree add -b cse-c2-validation-hardening-code wt/cse-c2-validation-hardening-code feat/agent-api-codex-stream-exec`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Add production code needed to support cross-platform fake-binary fixture validation (as pinned in `C2-spec.md`).
- Ensure redaction rules are upheld: emitted universal errors/events must not contain raw JSONL lines (including lines embedded in upstream `ExecStreamError` variants).
- Do not add/modify tests in this task.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## End Checklist
1. Run required commands (fmt/clippy) and capture outputs.
2. Commit worktree changes on branch `cse-c2-validation-hardening-code`.
3. Checkout `feat/agent-api-codex-stream-exec`; set `C2-code` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C2-code`).
4. Remove worktree `wt/cse-c2-validation-hardening-code`.

