# Kickoff Prompt — C0-integ (claude_code streaming API)

## Scope
- Merge C0 code + test branches, reconcile to spec, and run integration gates per:
  - `docs/project_management/next/claude-code-live-stream-json/C0-spec.md`
  - `docs/adr/0010-claude-code-live-stream-json.md`

## Start Checklist
1. `git checkout feat/claude-code-live-stream-json && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, and this prompt.
3. Set `C0-integ` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-integ`).
5. Create integration worktree: `git worktree add -b ccsj-c0-stream-api-integ wt/ccsj-c0-stream-api-integ feat/claude-code-live-stream-json`.
6. Do not edit docs/tasks/session_log.md from the worktree.

## Requirements
- Merge `ccsj-c0-stream-api-code` + `ccsj-c0-stream-api-test` into the integration worktree.
- Reconcile implementation to `C0-spec.md` and ADR-0010.
- Ensure the feature-local smoke workflow/scripts remain runnable on GitHub-hosted runners.

## Commands (required)
- `cargo fmt`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Relevant tests (at minimum, new/changed suites from C0-test)
- Linux-only gate: `make preflight`

## End Checklist
1. Commit integration changes on `ccsj-c0-stream-api-integ`.
2. Fast-forward merge `ccsj-c0-stream-api-integ` into `feat/claude-code-live-stream-json`.
3. Checkout `feat/claude-code-live-stream-json`; set `C0-integ` to `completed` in `tasks.json`; add END entry to `session_log.md`; commit docs (`docs: finish C0-integ`).
4. Remove worktree: `git worktree remove wt/ccsj-c0-stream-api-integ`.

