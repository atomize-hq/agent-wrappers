# Kickoff Prompt — C1-test (`agent_api` Claude live events)

## Scope
- Implement C1 tests per:
  - `docs/project_management/next/claude-code-live-stream-json/C1-spec.md`
  - `docs/adr/0010-claude-code-live-stream-json.md`
- Role boundary: tests/fixtures/harnesses only. No production code.

## Start Checklist
1. `git checkout feat/claude-code-live-stream-json && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C1-spec.md`, and this prompt.
3. Set `C1-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C1-test`).
5. Create worktree: `git worktree add -b ccsj-c1-agent-api-wiring-test wt/ccsj-c1-agent-api-wiring-test feat/claude-code-live-stream-json`.
6. Do not edit `docs/project_management/next/claude-code-live-stream-json/tasks.json` or `docs/project_management/next/claude-code-live-stream-json/session_log.md` from the worktree.

## Requirements
- Add synthetic tests proving:
  - Claude backend capabilities include `agent_api.events.live`
  - events can be observed before process exit
  - completion is gated per Universal Agent API DR-0012 (waits for stream finality or drop)
- Do not require a real `claude` binary.

## Commands (required)
- `cargo fmt`
- Targeted `cargo test ...` for suites added/updated (record exact commands in `session_log.md`).

## End Checklist
1. Run required commands; capture pass/fail in `session_log.md` END entry.
2. Commit changes from inside `wt/ccsj-c1-agent-api-wiring-test` (no docs/tasks/session_log edits).
3. Checkout `feat/claude-code-live-stream-json`; set `C1-test` to `completed` in `tasks.json`; add END entry to `session_log.md`; commit docs (`docs: finish C1-test`).
4. Remove worktree: `git worktree remove wt/ccsj-c1-agent-api-wiring-test`.
