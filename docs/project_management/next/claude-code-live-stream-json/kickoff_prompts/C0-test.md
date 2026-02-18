# Kickoff Prompt — C0-test (claude_code streaming API)

## Scope
- Implement C0 tests per:
  - `docs/project_management/next/claude-code-live-stream-json/C0-spec.md`
  - `docs/adr/0010-claude-code-live-stream-json.md`
- Role boundary: tests/fixtures/harnesses only. No production code.

## Start Checklist
1. `git checkout feat/claude-code-live-stream-json && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, and this prompt.
3. Set `C0-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-test`).
5. Create worktree: `git worktree add -b ccsj-c0-stream-api-test wt/ccsj-c0-stream-api-test feat/claude-code-live-stream-json`.
6. Do not edit `docs/project_management/next/claude-code-live-stream-json/tasks.json` or `docs/project_management/next/claude-code-live-stream-json/session_log.md` from the worktree.

## Requirements
- Add synthetic/fixture-based tests proving:
  - events can be yielded before process exit (incrementality)
  - CRLF + blank-line handling matches the protocol rules
  - parse errors are redacted (no raw line content)
- Do not rely on a real `claude` binary.

## Commands (required)
- `cargo fmt`
- Targeted `cargo test ...` for suites added/updated (record exact commands in `session_log.md`).

## End Checklist
1. Run required commands; capture pass/fail in `session_log.md` END entry.
2. Commit changes from inside `wt/ccsj-c0-stream-api-test` (no docs/tasks/session_log edits).
3. Checkout `feat/claude-code-live-stream-json`; set `C0-test` to `completed` in `tasks.json`; add END entry to `session_log.md`; commit docs (`docs: finish C0-test`).
4. Remove worktree: `git worktree remove wt/ccsj-c0-stream-api-test`.
