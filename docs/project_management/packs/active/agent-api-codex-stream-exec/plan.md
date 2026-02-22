# Agent API Codex `stream_exec` parity — Plan

## Purpose

Refactor the `agent_api` Codex backend to use `codex::CodexClient::stream_exec` while preserving:
- “live” streaming semantics (events are emitted before completion resolves)
- per-run environment override precedence (`AgentWrapperRunRequest.env`)
- deterministic exec-policy defaults + overrides (non-interactive + sandbox) via extensions
- universal safety guarantees (bounded messages; no raw JSONL line leakage)
- DR-0012 finality semantics (completion is final; event stream termination is ordered and deterministic)

This feature’s planning pack is canonical under:
`docs/project_management/packs/active/agent-api-codex-stream-exec/`

## Guardrails

- Triads only: code / test / integration. No mixed roles.
- Code: production code only; no tests. Required commands:
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Test: tests/fixtures/harnesses only; no production logic. Required commands:
  - `cargo fmt`
  - targeted `cargo test ...` for suites added/touched
- Integration: merges code+tests, reconciles to spec, and must run:
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - relevant tests
  - `make preflight` (Linux only)
- Docs/tasks/session_log edits happen only on the orchestration branch (`feat/agent-api-codex-stream-exec`), never from worktrees.
- Prefer fixture/fake-binary validation; do not require a real Codex install on CI.

## Branch & Worktree Conventions

- Orchestration branch: `feat/agent-api-codex-stream-exec`
- Feature prefix: `cse`
- Branch naming: `cse-<triad>-<scope>-<role>` (e.g., `cse-c1-codex-stream-exec-code`)
- Worktrees: `wt/<branch>` (in-repo; ignored by git)

## Triad Overview (aligned to `spec_manifest.md`)

- **C0 — Codex wrapper per-run env override API:** Add the minimum additive `crates/codex` API needed so `agent_api` can apply per-run environment overrides while still executing via `CodexClient::stream_exec`.
- **C1 — `agent_api` Codex backend refactor to `stream_exec`:** Consume `codex::CodexClient::stream_exec` and map typed Codex events (`ThreadEvent`) to `AgentWrapperEvent` live, preserving DR-0012 finality semantics.
- **C2 — Validation hardening (fixtures + safety):** Add fake-binary fixtures and integration tests proving (a) at least one event is emitted before completion resolves, (b) env precedence is preserved, (c) redaction rules prevent raw JSONL lines from leaking through universal errors/events, and (d) exec-policy defaults/overrides are honored deterministically.

## Start Checklist (all tasks)

1. `git checkout feat/agent-api-codex-stream-exec && git pull --ff-only`
2. Read: this plan, `tasks.json`, `session_log.md`, the relevant spec (`C*-spec.md`), and your kickoff prompt.
3. Set the task status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add a START entry to `session_log.md`; commit docs (`docs: start <task-id>`).
5. Create the task branch and worktree from `feat/agent-api-codex-stream-exec`:
   - `git worktree add -b <branch> wt/<branch> feat/agent-api-codex-stream-exec`
6. Do **not** edit docs/tasks/session_log from the worktree.

## End Checklist (code/test)

1. Run required commands (code: fmt + clippy; test: fmt + targeted tests) and capture outputs.
2. From inside the worktree, commit task branch changes (no docs/tasks/session_log edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed). Do **not** merge into `feat/agent-api-codex-stream-exec`.
4. Checkout `feat/agent-api-codex-stream-exec`; update `tasks.json` status; add an END entry to `session_log.md` with commands/results/blockers; commit docs (`docs: finish <task-id>`).
5. Remove the worktree: `git worktree remove wt/<branch>`.

## End Checklist (integration)

1. Merge code/test branches into the integration worktree; reconcile behavior to the spec.
2. Run (capture outputs):
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - relevant tests (at minimum, suites introduced by the triad’s test task)
   - Linux-only gate: `make preflight`
3. Commit integration changes to the integration branch.
4. Fast-forward merge the integration branch into `feat/agent-api-codex-stream-exec`; update `tasks.json` and `session_log.md` with the END entry; commit docs (`docs: finish <task-id>`).
5. Remove the worktree.

## Context Budget & Triad Sizing

- Keep each triad ≤ ~40–50% of a 272k token context window (~110–150k tokens).
- If the refactor expands (additional protocol surfaces, more platform deltas), split into additional triads rather than growing C0–C2.
