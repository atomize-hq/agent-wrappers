# Kickoff Prompt — C1-test (`agent_api` Codex backend `stream_exec` refactor)

## Scope
- Add tests only (no production logic) validating the C1 stream_exec adapter behavior per `C1-spec.md`.

## Start Checklist
1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C1-spec.md`, and this prompt.
3. Set `C1-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C1-test`).
5. Create the task branch and worktree: `git worktree add -b cse-c1-codex-stream-exec-test wt/cse-c1-codex-stream-exec-test feat/agent-api-codex-stream-exec`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Tests must not require a real Codex binary.
- Validate behavior pinned by:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/C1-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`
- No production code changes.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - targeted `cargo test ...` for suites you add/touch

## End Checklist
1. Run required commands (fmt + targeted tests) and capture outputs.
2. Commit worktree changes on branch `cse-c1-codex-stream-exec-test`.
3. Checkout `feat/agent-api-codex-stream-exec`; set `C1-test` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C1-test`).
4. Remove worktree `wt/cse-c1-codex-stream-exec-test`.

