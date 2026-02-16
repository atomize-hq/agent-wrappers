# Kickoff Prompt — C0-integ (Core universal API crate)

## Scope
- Merge `C0-code` + `C0-test`, resolve drift against `C0-spec.md` and the contract. Integration owns aligning code/tests to the spec.

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, and this prompt.
3. Set `C0-integ` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-integ`).
5. Create the integration branch and worktree: `git worktree add -b uaa-c0-core-integ wt/uaa-c0-core-integ feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Merge upstream code/test branches for C0 and reconcile to:
  - `docs/project_management/next/universal-agent-api/C0-spec.md`
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - relevant `cargo test ...` (at minimum, suites introduced by C0-test)
  - `make preflight`

## End Checklist
1. Merge the upstream C0 code/test branches into the integration worktree and reconcile behavior to the spec.
2. Run required commands and capture outputs.
3. Commit integration changes on branch `uaa-c0-core-integ`.
4. Fast-forward merge `uaa-c0-core-integ` into `feat/universal-agent-api`; set `C0-integ` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C0-integ`).
5. Remove worktree `wt/uaa-c0-core-integ`.

