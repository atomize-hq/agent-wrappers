# Kickoff Prompt – C0-test (<TRIAD TITLE>)

## Scope
- Tests/fixtures/harnesses only; no production code changes. Cover `C0-spec.md`.

## Start Checklist
1. `git checkout feat/claude-code-cli-parity-2.1.29 && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, this prompt.
3. Set `C0-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-test`).
5. Create the task branch and worktree: `git worktree add -b claude-code-cli-parity-2-1-29-c0-v2-1-29-test wt/claude-code-cli-parity-2-1-29-c0-v2-1-29-test feat/claude-code-cli-parity-2.1.29`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Add tests/fixtures validating C0 acceptance criteria; prefer deterministic fixtures over live external dependencies.
- Required commands:
  - `cargo fmt`
  - Targeted `cargo test ...` for suites you add/touch (record exact commands in END log).

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/claude-code-cli-parity-2-1-29-c0-v2-1-29-test`, commit C0-test changes (no docs/tasks/session_log.md edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed); do **not** merge into `feat/claude-code-cli-parity-2.1.29`.
4. Checkout `feat/claude-code-cli-parity-2.1.29`; update `tasks.json` to `completed`; add an END entry to `session_log.md` with commands/results/blockers; create downstream prompts if missing; commit docs (`docs: finish C0-test`).
5. Remove worktree `wt/claude-code-cli-parity-2-1-29-c0-v2-1-29-test`.


## Parity Work Queue (from coverage.any.json)
Report: `cli_manifests/claude_code/reports/2.1.29/coverage.any.json`

### Missing commands
- (none)

### Missing flags
- (none)

### Missing args
- (none)
