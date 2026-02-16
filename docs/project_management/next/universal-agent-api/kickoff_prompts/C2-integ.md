# Kickoff Prompt — C2-integ (Claude Code backend adapter)

## Scope
- Merge `C2-code` + `C2-test`, resolve drift against `C2-spec.md`. Integration owns aligning code/tests to the spec.

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C2-spec.md`, and this prompt.
3. Set `C2-integ` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C2-integ`).
5. Create the integration branch and worktree: `git worktree add -b uaa-c2-claude-integ wt/uaa-c2-claude-integ feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Reconcile behavior to:
  - `docs/project_management/next/universal-agent-api/C2-spec.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - relevant `cargo test ...`
  - `make preflight`

## End Checklist
1. Merge upstream C2 code/test branches into the integration worktree and reconcile to the spec.
2. Run required commands and capture outputs.
3. Commit integration changes on branch `uaa-c2-claude-integ`.
4. Fast-forward merge `uaa-c2-claude-integ` into `feat/universal-agent-api`; set `C2-integ` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C2-integ`).
5. Remove worktree `wt/uaa-c2-claude-integ`.

