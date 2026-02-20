# Kickoff Prompt — C2-integ (Validation hardening: fixtures + safety)

## Scope
- Merge `C2-code` + `C2-test`, resolve drift against `C2-spec.md`. Integration owns aligning code/tests to the spec.

## Start Checklist
1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C2-spec.md`, and this prompt.
3. Set `C2-integ` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C2-integ`).
5. Create the integration branch and worktree: `git worktree add -b cse-c2-validation-hardening-integ wt/cse-c2-validation-hardening-integ feat/agent-api-codex-stream-exec`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Merge upstream code/test branches for C2 and reconcile to:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/C2-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/platform-parity-spec.md`
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - relevant `cargo test ...` (at minimum, suites introduced by C2-test)
  - `make preflight` (Linux only)

## End Checklist
1. Merge the upstream C2 code/test branches into the integration worktree and reconcile behavior to the spec.
2. Run required commands and capture outputs.
3. Commit integration changes on branch `cse-c2-validation-hardening-integ`.
4. Fast-forward merge `cse-c2-validation-hardening-integ` into `feat/agent-api-codex-stream-exec`; set `C2-integ` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C2-integ`).
5. Remove worktree `wt/cse-c2-validation-hardening-integ`.

