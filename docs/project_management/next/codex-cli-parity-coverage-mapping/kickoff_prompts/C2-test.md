# Kickoff Prompt – C2-test (Wrapper coverage generator)

## Scope
- Tests only; no production code. Add tests for wrapper coverage generator + scope resolution semantics per `docs/project_management/next/codex-cli-parity-coverage-mapping/C2-spec.md`.

## Start Checklist
1. `git checkout feat/codex-cli-parity-coverage-mapping && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity-coverage-mapping/plan.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`, `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/C2-spec.md`, this prompt.
3. Set `C2-test` status to `in_progress` in `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`; commit docs (`docs: start C2-test`).
5. Create the task branch and worktree: `git worktree add -b ccm-c2-wrapper-coverage-test wt/ccm-c2-wrapper-coverage-test feat/codex-cli-parity-coverage-mapping`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` or `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` from the worktree.

## Requirements
- Implement C2 tests per `docs/project_management/next/codex-cli-parity-coverage-mapping/C2-spec.md`.
- Required commands (before handoff):
  - `cargo fmt`
  - Targeted `cargo test ...` for the suites you add/touch (capture outputs).

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccm-c2-wrapper-coverage-test`, commit C2-test changes (no planning-pack docs edits).
3. From outside the worktree, ensure branch `ccm-c2-wrapper-coverage-test` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity-coverage-mapping`.
4. Checkout `feat/codex-cli-parity-coverage-mapping`; update tasks/session log; commit docs (`docs: finish C2-test`).
5. Remove worktree `wt/ccm-c2-wrapper-coverage-test`.

