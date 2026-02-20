# Kickoff Prompt — C0-test (Codex wrapper per-run env override API)

## Scope
- Add tests only (no production logic) for the C0 env override behavior.

## Start Checklist
1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, and this prompt.
3. Set `C0-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-test`).
5. Create the task branch and worktree: `git worktree add -b cse-c0-codex-env-test wt/cse-c0-codex-env-test feat/agent-api-codex-stream-exec`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Tests must not require a real Codex binary.
- Validate the env precedence/absence semantics pinned by:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/C0-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`
- No production code changes.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - targeted `cargo test ...` for suites you add/touch

## End Checklist
1. Run required commands (fmt + targeted tests) and capture outputs.
2. Commit worktree changes on branch `cse-c0-codex-env-test`.
3. Checkout `feat/agent-api-codex-stream-exec`; set `C0-test` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C0-test`).
4. Remove worktree `wt/cse-c0-codex-env-test`.

