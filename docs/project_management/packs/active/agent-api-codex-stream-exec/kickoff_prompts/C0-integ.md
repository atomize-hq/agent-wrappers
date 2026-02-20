# Kickoff Prompt — C0-integ (Codex wrapper per-run env override API)

## Scope
- Merge `C0-code` + `C0-test`, resolve drift against `C0-spec.md` and the contract. Integration owns aligning code/tests to the spec.

## Start Checklist
1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, and this prompt.
3. Set `C0-integ` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-integ`).
5. Create the integration branch and worktree: `git worktree add -b cse-c0-codex-env-integ wt/cse-c0-codex-env-integ feat/agent-api-codex-stream-exec`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Merge upstream code/test branches for C0 and reconcile to:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/C0-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/platform-parity-spec.md`
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - relevant `cargo test ...` (at minimum, suites introduced by C0-test)
  - `make preflight` (Linux only)

## End Checklist
1. Merge the upstream C0 code/test branches into the integration worktree and reconcile behavior to the spec.
2. Run required commands and capture outputs.
3. Commit integration changes on branch `cse-c0-codex-env-integ`.
4. Fast-forward merge `cse-c0-codex-env-integ` into `feat/agent-api-codex-stream-exec`; set `C0-integ` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C0-integ`).
5. Remove worktree `wt/cse-c0-codex-env-integ`.

