# Kickoff Prompt – C0-integ (Snapshot schema + generator)

## Scope
Merge `C0-code` + `C0-test`, reconcile to `docs/project_management/next/codex-cli-parity/C0-spec.md`, and ensure the triad is green. Integration owns aligning code/tests to the spec.

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C0-spec.md`, this prompt.
3. Set `C0-integ` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C0-integ`).
5. Create the integration branch and worktree: `git worktree add -b ccp-c0-snapshot-integ wt/ccp-c0-snapshot-integ feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Merge branches `ccp-c0-snapshot-code` + `ccp-c0-snapshot-test` and reconcile behavior to `docs/project_management/next/codex-cli-parity/C0-spec.md`.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p xtask`
  - Integration gate: `make preflight`

## End Checklist
1. Merge the upstream C0 code/test branches into the integration worktree and reconcile behavior to the spec.
2. Run required commands (fmt/clippy/tests/integration gate) and capture outputs.
3. Commit integration changes on branch `ccp-c0-snapshot-integ`.
4. Fast-forward merge `ccp-c0-snapshot-integ` into `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C0-integ`).
5. Remove worktree `wt/ccp-c0-snapshot-integ`.
