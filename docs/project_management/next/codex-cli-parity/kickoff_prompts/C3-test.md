# Kickoff Prompt – C3-test (Ops playbook + promotion rules)

## Scope
Add any tests/fixtures needed by `docs/project_management/next/codex-cli-parity/C3-spec.md` changes (tests only). Do not change production code.

Role boundaries:
- Test agent: tests only; no production logic changes.

Note (no ambiguity):
- C3 is documentation-only. This task is expected to be a no-op unless C3-code introduces something that genuinely needs tests. If no test work exists, do not create commits; record “no-op” + reasoning in the END entry.

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C3-spec.md`, this prompt.
3. Set `C3-test` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C3-test`).
5. Create the task branch and worktree: `git worktree add -b ccp-c3-ops-test wt/ccp-c3-ops-test feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Implement tests per `docs/project_management/next/codex-cli-parity/C3-spec.md` as applicable.
- Required commands:
  - `cargo fmt`
  - `cargo test -p codex` (record exact command(s) in END log)

## End Checklist
1. If you made changes: run the required commands above and capture their outputs.
2. If you made changes: inside `wt/ccp-c3-ops-test`, commit C3-test changes (no planning-pack docs edits).
3. If you made changes: from outside the worktree, ensure branch `ccp-c3-ops-test` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity`.
4. If no changes were needed: do not create commits; record a no-op END entry with reasoning.
5. Checkout `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: finish C3-test`).
6. Remove worktree `wt/ccp-c3-ops-test`.
