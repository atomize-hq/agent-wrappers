# Kickoff Prompt – C0-integ (Deterministic validators)

## Scope
- Integration: merge C0 code + test branches, reconcile to spec, and gate with fmt/clippy/tests/preflight.

## Start Checklist
1. `git checkout feat/codex-cli-parity-coverage-mapping && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity-coverage-mapping/plan.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`, `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/C0-spec.md`, this prompt.
3. Set `C0-integ` status to `in_progress` in `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`; commit docs (`docs: start C0-integ`).
5. Create the integration branch and worktree: `git worktree add -b ccm-c0-validate-integ wt/ccm-c0-validate-integ feat/codex-cli-parity-coverage-mapping`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` or `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` from the worktree.

## Requirements
- Merge `ccm-c0-validate-code` and `ccm-c0-validate-test` into the integration worktree and reconcile to `docs/project_management/next/codex-cli-parity-coverage-mapping/C0-spec.md`.
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - Relevant tests (at minimum, the suites introduced by C0-test)
  - `make preflight`

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccm-c0-validate-integ`, commit C0 integration changes.
3. Fast-forward merge `ccm-c0-validate-integ` into `feat/codex-cli-parity-coverage-mapping`; update tasks/session log; commit docs (`docs: finish C0-integ`).
4. Remove worktree `wt/ccm-c0-validate-integ`.

