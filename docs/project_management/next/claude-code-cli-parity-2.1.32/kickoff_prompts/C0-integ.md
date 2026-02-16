# Kickoff Prompt – C0-integ (<TRIAD TITLE>)

## Scope
- Merge `C0-code` + `C0-test`, resolve drift against `C0-spec.md`, and ensure the triad is green. Integration owns aligning code/tests to the spec.

## Start Checklist
1. `git checkout feat/claude-code-cli-parity-2.1.32 && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, this prompt.
3. Set `C0-integ` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-integ`).
5. Create the integration branch and worktree: `git worktree add -b claude-code-cli-parity-2-1-32-c0-v2-1-32-integ wt/claude-code-cli-parity-2-1-32-c0-v2-1-32-integ feat/claude-code-cli-parity-2.1.32`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Merge the upstream code/test branches for C0, reconcile behavior to `C0-spec.md`.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - Relevant tests (at minimum, the suites introduced by C0-test)
  - Integration gate: `make preflight`

## End Checklist
1. Merge the upstream C0 code/test branches into the integration worktree and reconcile behavior to the spec.
2. Run required commands (fmt/clippy/tests/integration gate) and capture outputs.
3. Commit integration changes on branch `claude-code-cli-parity-2-1-32-c0-v2-1-32-integ`.
4. Fast-forward merge the integration branch into `feat/claude-code-cli-parity-2.1.32`; update `tasks.json` to `completed`; add an END entry to `session_log.md` with commands/results/blockers; commit docs (`docs: finish C0-integ`).
5. Remove worktree `wt/claude-code-cli-parity-2-1-32-c0-v2-1-32-integ`.


## Parity Work Queue (from coverage.any.json)
Report: `cli_manifests/claude_code/reports/2.1.32/coverage.any.json`

### Missing commands
- (none)

### Missing flags
- `mcp add --callback-port`
- `mcp add --client-id`
- `mcp add --client-secret`
- `mcp add-json --client-secret`

### Missing args
- (none)
