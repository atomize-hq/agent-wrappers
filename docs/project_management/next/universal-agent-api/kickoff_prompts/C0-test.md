# Kickoff Prompt — C0-test (Core universal API crate)

## Scope
- Add tests only (no production logic) for the C0 core contract.

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, and this prompt.
3. Set `C0-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-test`).
5. Create the task branch and worktree: `git worktree add -b uaa-c0-core-test wt/uaa-c0-core-test feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Tests must not require real Codex/Claude binaries.
- Prefer minimal unit tests in `crates/agent_api` and fixture-based mapping tests if needed.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - targeted `cargo test ...` for suites you add/touch

## End Checklist
1. Run required commands (fmt + targeted tests) and capture outputs.
2. Commit worktree changes on branch `uaa-c0-core-test`.
3. Checkout `feat/universal-agent-api`; set `C0-test` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C0-test`).
4. Remove worktree `wt/uaa-c0-core-test`.

