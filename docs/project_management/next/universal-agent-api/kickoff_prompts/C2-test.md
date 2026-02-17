# Kickoff Prompt — C2-test (Claude Code backend adapter)

## Scope
- Add tests only (no production logic) validating Claude backend mapping per `C2-spec.md`.

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C2-spec.md`, and this prompt.
3. Set `C2-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C2-test`).
5. Create the task branch and worktree: `git worktree add -b uaa-c2-claude-test wt/uaa-c2-claude-test feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Tests must not require a real `claude` binary.
- Prefer fixture-based stream-json samples mapped into `AgentWrapperEvent`.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - targeted `cargo test ...`

## End Checklist
1. Run required commands and capture outputs.
2. Commit worktree changes on branch `uaa-c2-claude-test`.
3. Checkout `feat/universal-agent-api`; set `C2-test` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C2-test`).
4. Remove worktree `wt/uaa-c2-claude-test`.
