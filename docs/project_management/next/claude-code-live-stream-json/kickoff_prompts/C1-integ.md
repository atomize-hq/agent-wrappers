# Kickoff Prompt — C1-integ (`agent_api` Claude live events)

## Scope
- Merge C1 code + test branches, reconcile to spec, and run integration gates per:
  - `docs/project_management/next/claude-code-live-stream-json/C1-spec.md`
  - `docs/adr/0010-claude-code-live-stream-json.md`

## Start Checklist
1. `git checkout feat/claude-code-live-stream-json && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C1-spec.md`, and this prompt.
3. Set `C1-integ` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C1-integ`).
5. Create integration worktree: `git worktree add -b ccsj-c1-agent-api-wiring-integ wt/ccsj-c1-agent-api-wiring-integ feat/claude-code-live-stream-json`.
6. Do not edit docs/tasks/session_log.md from the worktree.

## Requirements
- Merge `ccsj-c1-agent-api-wiring-code` + `ccsj-c1-agent-api-wiring-test` into the integration worktree.
- Reconcile implementation to `C1-spec.md` and ADR-0010.
- Ensure DR-0012 completion semantics are preserved and raw-line safety rules hold.

## Commands (required)
- `cargo fmt`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Relevant tests (at minimum, new/changed suites from C1-test)
- Linux-only gate: `make preflight`

## End Checklist
1. Commit integration changes on `ccsj-c1-agent-api-wiring-integ`.
2. Fast-forward merge `ccsj-c1-agent-api-wiring-integ` into `feat/claude-code-live-stream-json`.
3. Checkout `feat/claude-code-live-stream-json`; set `C1-integ` to `completed` in `tasks.json`; add END entry to `session_log.md`; commit docs (`docs: finish C1-integ`).
4. Remove worktree: `git worktree remove wt/ccsj-c1-agent-api-wiring-integ`.

