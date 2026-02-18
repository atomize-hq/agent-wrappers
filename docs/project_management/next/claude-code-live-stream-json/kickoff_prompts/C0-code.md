# Kickoff Prompt — C0-code (claude_code streaming API)

## Scope
- Implement C0 production code per:
  - `docs/project_management/next/claude-code-live-stream-json/C0-spec.md`
  - `docs/adr/0010-claude-code-live-stream-json.md`
- Role boundary: production code + CI workflow/scripts only. No tests.

## Start Checklist
1. `git checkout feat/claude-code-live-stream-json && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, and this prompt.
3. Set `C0-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-code`).
5. Create worktree: `git worktree add -b ccsj-c0-stream-api-code wt/ccsj-c0-stream-api-code feat/claude-code-live-stream-json`.
6. Do not edit docs/tasks/session_log.md from the worktree.

## Requirements
- Implement the streaming API surface described in ADR-0010.
- Preserve safety posture: do not retain or emit raw backend lines; parse errors must be redacted.
- Add the dedicated workflow + smoke scripts referenced by `ci_checkpoint_plan.md`:
  - `.github/workflows/claude-code-live-stream-json-smoke.yml`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/*`

## Commands (required)
- `cargo fmt`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## End Checklist
1. Run required commands; capture pass/fail in `session_log.md` END entry.
2. Commit changes from inside `wt/ccsj-c0-stream-api-code` (no docs/tasks/session_log edits).
3. Checkout `feat/claude-code-live-stream-json`; set `C0-code` to `completed` in `tasks.json`; add END entry to `session_log.md`; commit docs (`docs: finish C0-code`).
4. Remove worktree: `git worktree remove wt/ccsj-c0-stream-api-code`.

