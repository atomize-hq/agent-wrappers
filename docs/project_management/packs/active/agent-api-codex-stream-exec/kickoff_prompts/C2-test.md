# Kickoff Prompt — C2-test (Validation hardening: fixtures + safety)

## Scope
- Add tests only (no production logic) proving:
  - at least one event is emitted before completion resolves (“live” evidence)
  - env precedence is preserved
  - redaction rules prevent raw JSONL lines from leaking through universal errors/events

## Start Checklist
1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C2-spec.md`, and this prompt.
3. Set `C2-test` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C2-test`).
5. Create the task branch and worktree: `git worktree add -b cse-c2-validation-hardening-test wt/cse-c2-validation-hardening-test feat/agent-api-codex-stream-exec`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Tests must not require a real Codex binary.
- Follow the fake-binary/fixture strategy pinned by:
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/C2-spec.md`
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/platform-parity-spec.md`
- Add explicit assertions for redaction (no raw JSONL line leakage).
- No production code changes.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - targeted `cargo test ...` for suites you add/touch

## End Checklist
1. Run required commands (fmt + targeted tests) and capture outputs.
2. Commit worktree changes on branch `cse-c2-validation-hardening-test`.
3. Checkout `feat/agent-api-codex-stream-exec`; set `C2-test` to `completed`; add END entry to `session_log.md`; commit docs (`docs: finish C2-test`).
4. Remove worktree `wt/cse-c2-validation-hardening-test`.

