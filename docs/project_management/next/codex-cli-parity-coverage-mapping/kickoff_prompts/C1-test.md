# Kickoff Prompt – C1-test (Per-target snapshots + union)

## Scope
- Tests only; no production code. Add tests/fixtures for per-target snapshot output and union merge semantics per `docs/project_management/next/codex-cli-parity-coverage-mapping/C1-spec.md`.

## Start Checklist
1. `git checkout feat/codex-cli-parity-coverage-mapping && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity-coverage-mapping/plan.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`, `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/C1-spec.md`, this prompt.
3. Set `C1-test` status to `in_progress` in `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`; commit docs (`docs: start C1-test`).
5. Create the task branch and worktree: `git worktree add -b ccm-c1-union-test wt/ccm-c1-union-test feat/codex-cli-parity-coverage-mapping`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` or `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` from the worktree.

## Requirements
- Implement C1 tests per `docs/project_management/next/codex-cli-parity-coverage-mapping/C1-spec.md`.
- Required commands (before handoff):
  - `cargo fmt`
  - Targeted `cargo test ...` for the suites you add/touch (capture outputs).

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccm-c1-union-test`, commit C1-test changes (no planning-pack docs edits).
3. From outside the worktree, ensure branch `ccm-c1-union-test` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity-coverage-mapping`.
4. Checkout `feat/codex-cli-parity-coverage-mapping`; update tasks/session log; commit docs (`docs: finish C1-test`).
5. Remove worktree `wt/ccm-c1-union-test`.

