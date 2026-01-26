# Kickoff Prompt – C2-integ (JSONL + notifications compatibility)

## Scope
Merge `C2-code` + `C2-test`, reconcile to `docs/project_management/next/codex-cli-parity/C2-spec.md`, and ensure the triad is green. Integration owns aligning code/tests to the spec.

Expected deliverables (exact paths):
- Fixtures: `crates/codex/examples/fixtures/versioned/`
- Tests: `crates/codex/tests/jsonl_compat.rs`
- Compat docs: `crates/codex/JSONL_COMPAT.md`

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C2-spec.md`, this prompt.
3. Set `C2-integ` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C2-integ`).
5. Create the integration branch and worktree: `git worktree add -b ccp-c2-jsonl-integ wt/ccp-c2-jsonl-integ feat/codex-cli-parity`.
6. Do **not** edit docs/tasks/session_log from the worktree.

## Requirements
- Merge branches `ccp-c2-jsonl-code` + `ccp-c2-jsonl-test` and reconcile behavior to `docs/project_management/next/codex-cli-parity/C2-spec.md`.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p codex`
  - Integration gate: `make preflight`

## End Checklist
1. Merge upstream C2 code/test branches into the integration worktree and reconcile behavior to the spec.
2. Run required commands above and capture outputs.
3. Commit integration changes on branch `ccp-c2-jsonl-integ`.
4. Fast-forward merge `ccp-c2-jsonl-integ` into `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C2-integ`).
5. Remove worktree `wt/ccp-c2-jsonl-integ`.
