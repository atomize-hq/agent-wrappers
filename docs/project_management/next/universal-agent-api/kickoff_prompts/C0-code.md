# Kickoff Prompt — C0-code (Core universal API crate)

## Scope
- Implement C0 production code only (no tests) per:
  - `docs/project_management/next/universal-agent-api/C0-spec.md`
  - `docs/project_management/next/universal-agent-api/contract.md`

## Start Checklist
1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `decision_register.md`, `C0-spec.md`, and this prompt.
3. Set `C0-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-code`).
5. Create the task branch and worktree: `git worktree add -b uaa-c0-core-code wt/uaa-c0-core-code feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Implement `crates/agent_api` core types/traits/gateway/errors per the contract.
- Implement the GitHub-hosted smoke workflow required by the CI checkpoint plan:
  - `.github/workflows/universal-agent-api-smoke.yml` (runs the feature-local smoke scripts on `ubuntu-latest`, `macos-latest`, `windows-latest` and runs `make preflight` on Linux)
- Ensure the crate builds with default features (no backends enabled).
- Do not add/modify tests in this task.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## End Checklist
1. Run required commands (fmt/clippy) and capture outputs.
2. Commit worktree changes on branch `uaa-c0-core-code`.
3. Checkout `feat/universal-agent-api`; set `C0-code` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C0-code`).
4. Remove worktree `wt/uaa-c0-core-code`.
