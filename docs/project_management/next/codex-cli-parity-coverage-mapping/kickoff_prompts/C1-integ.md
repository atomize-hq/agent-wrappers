# Kickoff Prompt – C1-integ (Per-target snapshots + union)

## Scope
- Integration: merge C1 code + test branches, reconcile to spec, and gate with fmt/clippy/tests/preflight.

## Start Checklist
1. `git checkout feat/codex-cli-parity-coverage-mapping && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity-coverage-mapping/plan.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`, `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/C1-spec.md`, this prompt.
3. Set `C1-integ` status to `in_progress` in `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`; commit docs (`docs: start C1-integ`).
5. Create the integration branch and worktree: `git worktree add -b ccm-c1-union-integ wt/ccm-c1-union-integ feat/codex-cli-parity-coverage-mapping`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` or `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` from the worktree.

## Requirements
- Merge branches `ccm-c1-union-code` + `ccm-c1-union-test` and reconcile behavior to `docs/project_management/next/codex-cli-parity-coverage-mapping/C1-spec.md`.
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p xtask`
  - `make preflight`

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccm-c1-union-integ`, commit C1 integration changes.
3. Fast-forward merge `ccm-c1-union-integ` into `feat/codex-cli-parity-coverage-mapping`; update `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` with commands/results/blockers; commit docs (`docs: finish C1-integ`).
4. Remove worktree `wt/ccm-c1-union-integ`.
