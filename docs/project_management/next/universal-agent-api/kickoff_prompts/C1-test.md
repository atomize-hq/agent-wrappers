# Kickoff Prompt — C1-test (Codex backend adapter)

## Scope
- Add tests only (no production logic) validating Codex backend mapping per `C1-spec.md`.

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C1-spec.md`, and this prompt.
3. Set `C1-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C1-test`).
5. Create the task branch and worktree: `git worktree add -b uaa-c1-codex-test wt/uaa-c1-codex-test feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Tests must not require a real Codex binary.
- Prefer fixture-based Codex `ThreadEvent` samples mapped into `AgentEvent`.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - targeted `cargo test ...`

## End Checklist
1. Run required commands and capture outputs.
2. Commit worktree changes on branch `uaa-c1-codex-test`.
3. Checkout `feat/universal-agent-api`; set `C1-test` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C1-test`).
4. Remove worktree `wt/uaa-c1-codex-test`.

