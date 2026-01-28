# Kickoff Prompt – C2-integ (Wrapper coverage generator)

## Scope
- Integration: merge C2 code + test branches, reconcile to spec, and gate with fmt/clippy/tests/preflight.

## Start Checklist
1. `git checkout feat/codex-cli-parity-coverage-mapping && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity-coverage-mapping/plan.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`, `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/C2-spec.md`, this prompt.
3. Set `C2-integ` status to `in_progress` in `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`; commit docs (`docs: start C2-integ`).
5. Create the integration branch and worktree: `git worktree add -b ccm-c2-wrapper-coverage-integ wt/ccm-c2-wrapper-coverage-integ feat/codex-cli-parity-coverage-mapping`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` or `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` from the worktree.

## Requirements
- Merge branches `ccm-c2-wrapper-coverage-code` + `ccm-c2-wrapper-coverage-test` and reconcile behavior to `docs/project_management/next/codex-cli-parity-coverage-mapping/C2-spec.md`.
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p xtask`
  - `cargo test -p codex`
  - `make preflight`

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccm-c2-wrapper-coverage-integ`, commit C2 integration changes.
3. Fast-forward merge `ccm-c2-wrapper-coverage-integ` into `feat/codex-cli-parity-coverage-mapping`; update `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` with commands/results/blockers; commit docs (`docs: finish C2-integ`).
4. Remove worktree `wt/ccm-c2-wrapper-coverage-integ`.
