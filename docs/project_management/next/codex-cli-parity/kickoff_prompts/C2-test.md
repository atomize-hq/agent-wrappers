# Kickoff Prompt – C2-test (JSONL + notifications compatibility)

## Scope
Add fixtures + tests (tests only) that validate `docs/project_management/next/codex-cli-parity/C2-spec.md`, including drift-tolerant parsing and non-fatal error handling. Do not change production code.

Role boundaries:
- Test agent: tests only; no production logic changes.

Expected deliverables (exact paths):
- Fixtures: `crates/codex/examples/fixtures/versioned/`
- Tests: `crates/codex/tests/jsonl_compat.rs`

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C2-spec.md`, this prompt.
3. Set `C2-test` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C2-test`).
5. Create the task branch and worktree: `git worktree add -b ccp-c2-jsonl-test wt/ccp-c2-jsonl-test feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Implement tests/fixtures per `docs/project_management/next/codex-cli-parity/C2-spec.md`.
- Required commands:
  - `cargo fmt`
  - `cargo test -p codex` (record exact command(s) in END log)

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccp-c2-jsonl-test`, commit C2-test changes (no planning-pack docs edits).
3. From outside the worktree, ensure branch `ccp-c2-jsonl-test` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity`.
4. Checkout `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C2-test`).
5. Remove worktree `wt/ccp-c2-jsonl-test`.
