# Kickoff Prompt – C3-integ (Reports + metadata + pruning)

## Scope
- Integration: merge C3 code + test branches, reconcile to spec, and gate with fmt/clippy/tests/preflight.

## Start Checklist
1. `git checkout feat/codex-cli-parity-coverage-mapping && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity-coverage-mapping/plan.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`, `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/C3-spec.md`, this prompt.
3. Set `C3-integ` status to `in_progress` in `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`; commit docs (`docs: start C3-integ`).
5. Create the integration branch and worktree: `git worktree add -b ccm-c3-reports-integ wt/ccm-c3-reports-integ feat/codex-cli-parity-coverage-mapping`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` or `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` from the worktree.

## Requirements
- Merge `ccm-c3-reports-code` and `ccm-c3-reports-test` into the integration worktree and reconcile to `docs/project_management/next/codex-cli-parity-coverage-mapping/C3-spec.md`.
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - Relevant tests
  - `make preflight`

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccm-c3-reports-integ`, commit C3 integration changes.
3. Fast-forward merge `ccm-c3-reports-integ` into `feat/codex-cli-parity-coverage-mapping`; update tasks/session log; commit docs (`docs: finish C3-integ`).
4. Remove worktree `wt/ccm-c3-reports-integ`.

