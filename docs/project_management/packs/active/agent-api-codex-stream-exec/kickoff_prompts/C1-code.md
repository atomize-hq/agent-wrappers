# Kickoff Prompt — C1-code (`agent_api` Codex backend `stream_exec` refactor)

## Scope
- Implement C1 production code only (no tests) per:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/C1-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`

## Start Checklist
1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `decision_register.md`, `C1-spec.md`, and this prompt.
3. Set `C1-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C1-code`).
5. Create the task branch and worktree: `git worktree add -b cse-c1-codex-stream-exec-code wt/cse-c1-codex-stream-exec-code feat/agent-api-codex-stream-exec`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Refactor `agent_api` Codex backend to execute via `codex::CodexClient::stream_exec`.
- Map typed Codex events (`ThreadEvent`) into `AgentWrapperEvent` and preserve “live” semantics (events emitted before completion resolves) per the protocol spec.
- Preserve DR-0012 finality semantics (completion is final; event stream termination is ordered).
- Implement redaction mapping so universal errors/events do not leak raw JSONL lines.
- Do not add/modify tests in this task.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## End Checklist
1. Run required commands (fmt/clippy) and capture outputs.
2. Commit worktree changes on branch `cse-c1-codex-stream-exec-code`.
3. Checkout `feat/agent-api-codex-stream-exec`; set `C1-code` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C1-code`).
4. Remove worktree `wt/cse-c1-codex-stream-exec-code`.

