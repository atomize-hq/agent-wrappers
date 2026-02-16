# Kickoff Prompt — C1-code (Codex backend adapter)

## Scope
- Implement C1 production code only (no tests) per `C1-spec.md`.

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C1-spec.md`, and this prompt.
3. Set `C1-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C1-code`).
5. Create the task branch and worktree: `git worktree add -b uaa-c1-codex-code wt/uaa-c1-codex-code feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Implement the `codex` feature-gated backend on `agent_api` via `crates/codex`.
- Ensure `agent_api` still compiles without the feature enabled.
- Do not add/modify tests in this task.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## End Checklist
1. Run required commands and capture outputs.
2. Commit worktree changes on branch `uaa-c1-codex-code`.
3. Checkout `feat/universal-agent-api`; set `C1-code` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C1-code`).
4. Remove worktree `wt/uaa-c1-codex-code`.
