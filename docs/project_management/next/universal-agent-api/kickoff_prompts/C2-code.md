# Kickoff Prompt — C2-code (Claude Code backend adapter)

## Scope
- Implement C2 production code only (no tests) per `C2-spec.md`.

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C2-spec.md`, and this prompt.
3. Set `C2-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C2-code`).
5. Create the task branch and worktree: `git worktree add -b uaa-c2-claude-code wt/uaa-c2-claude-code feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Implement the `claude_code` feature-gated backend on `agent_api` via `crates/claude_code`.
- Buffered event production is acceptable; reflect semantics via capabilities (see DR-0001).
- Do not add/modify tests in this task.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## End Checklist
1. Run required commands and capture outputs.
2. Commit worktree changes on branch `uaa-c2-claude-code`.
3. Checkout `feat/universal-agent-api`; set `C2-code` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C2-code`).
4. Remove worktree `wt/uaa-c2-claude-code`.
