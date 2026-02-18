# Kickoff Prompt — C1-code (`agent_api` Claude live events)

## Scope
- Implement C1 production code per:
  - `docs/project_management/next/claude-code-live-stream-json/C1-spec.md`
  - `docs/adr/0010-claude-code-live-stream-json.md`
- Role boundary: production code only. No tests.

## Start Checklist
1. `git checkout feat/claude-code-live-stream-json && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C1-spec.md`, and this prompt.
3. Set `C1-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C1-code`).
5. Create worktree: `git worktree add -b ccsj-c1-agent-api-wiring-code wt/ccsj-c1-agent-api-wiring-code feat/claude-code-live-stream-json`.
6. Do not edit `docs/project_management/next/claude-code-live-stream-json/tasks.json` or `docs/project_management/next/claude-code-live-stream-json/session_log.md` from the worktree.

## Requirements
- Wire the Claude backend to the streaming API and emit events live.
- Advertise `agent_api.events.live`.
- Preserve Universal Agent API DR-0012 completion gating and safety posture (no raw backend lines).

## Commands (required)
- `cargo fmt`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## End Checklist
1. Run required commands; capture pass/fail in `session_log.md` END entry.
2. Commit changes from inside `wt/ccsj-c1-agent-api-wiring-code` (no docs/tasks/session_log edits).
3. Checkout `feat/claude-code-live-stream-json`; set `C1-code` to `completed` in `tasks.json`; add END entry to `session_log.md`; commit docs (`docs: finish C1-code`).
4. Remove worktree: `git worktree remove wt/ccsj-c1-agent-api-wiring-code`.
