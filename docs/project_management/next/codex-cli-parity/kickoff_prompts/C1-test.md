# Kickoff Prompt – C1-test (Version policy + CI workflows)

## Scope
Add tests/fixtures/harnesses (tests only) that validate `docs/project_management/next/codex-cli-parity/C1-spec.md` as applicable. Do not change production code.

Role boundaries:
- Test agent: tests only; no production logic changes; no live/credentialed flows.

Note (no ambiguity):
- C1 is primarily workflows/CI. If there is no meaningful test-only work, treat this task as an explicit no-op: do not create commits; record “no-op” + reasoning in the END entry.

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C1-spec.md`, this prompt.
3. Set `C1-test` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C1-test`).
5. Create the task branch and worktree: `git worktree add -b ccp-c1-validation-test wt/ccp-c1-validation-test feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Implement tests per `docs/project_management/next/codex-cli-parity/C1-spec.md`.
- Required commands:
  - `cargo fmt`
  - `cargo test -p codex` (record exact command(s) in END log)

## End Checklist
1. If you made changes: run the required commands above and capture their outputs.
2. If you made changes: inside `wt/ccp-c1-validation-test`, commit C1-test changes (no planning-pack docs edits).
3. If you made changes: from outside the worktree, ensure branch `ccp-c1-validation-test` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity`.
4. If no changes were needed: do not create commits; record a no-op END entry with reasoning.
5. Checkout `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: finish C1-test`).
6. Remove worktree `wt/ccp-c1-validation-test`.
