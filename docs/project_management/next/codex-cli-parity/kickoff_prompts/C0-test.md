# Kickoff Prompt – C0-test (Snapshot schema + generator)

## Scope
Add tests/fixtures (tests only) that validate `docs/project_management/next/codex-cli-parity/C0-spec.md`. Do not change non-test code.

Expected deliverables (exact paths):
- Tests: `crates/xtask/tests/`
- Test-only fixtures (if needed): `crates/xtask/tests/fixtures/`

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C0-spec.md`, this prompt.
3. Set `C0-test` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C0-test`).
5. Create the task branch and worktree: `git worktree add -b ccp-c0-snapshot-test wt/ccp-c0-snapshot-test feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Add tests/fixtures validating C0 acceptance criteria (as applicable); prefer deterministic fixtures over live external dependencies.
- Required commands:
  - `cargo fmt`
  - `cargo test -p xtask` (covers tests you add/touch; record exact command(s) in END log)

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccp-c0-snapshot-test`, commit C0-test changes (no planning-pack docs edits).
3. From outside the worktree, ensure branch `ccp-c0-snapshot-test` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity`.
4. Checkout `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C0-test`).
5. Remove worktree `wt/ccp-c0-snapshot-test`.
