# Kickoff Prompt — C0-code (Codex wrapper per-run env override API)

## Scope
- Implement C0 production code only (no tests) per:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/C0-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`

## Start Checklist
1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `decision_register.md`, `C0-spec.md`, and this prompt.
3. Set `C0-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-code`).
5. Create the task branch and worktree: `git worktree add -b cse-c0-codex-env-code wt/cse-c0-codex-env-code feat/agent-api-codex-stream-exec`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Add the minimum additive `crates/codex` API needed for `agent_api` to apply per-run env overrides while still running through `CodexClient::stream_exec`.
- Preserve precedence and non-leakage semantics pinned by:
  - `contract.md`
  - `decision_register.md` (A/B selection for env override strategy)
- Do not introduce behavior that requires a real Codex install on CI.
- Do not add/modify tests in this task.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## End Checklist
1. Run required commands (fmt/clippy) and capture outputs.
2. Commit worktree changes on branch `cse-c0-codex-env-code`.
3. Checkout `feat/agent-api-codex-stream-exec`; set `C0-code` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C0-code`).
4. Remove worktree `wt/cse-c0-codex-env-code`.

