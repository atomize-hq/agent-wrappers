# Claude Code live stream-json — Plan

## Purpose

Make Claude Code runs “live” under the Universal Agent API by adding a streaming `--print
--output-format stream-json` API to `crates/claude_code`, then wiring that API into `crates/agent_api`
so callers receive `AgentWrapperEvent`s before the process exits.

## Guardrails

- Triads only: code / test / integration. No mixed roles.
- Code: production code only; no tests. Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Test: tests/fixtures/harnesses only; no production logic. Required commands: `cargo fmt`; targeted `cargo test ...` for suites added/touched.
- Integration: merges code+tests, reconciles to spec, and must run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, relevant tests, and the repo gate: `make preflight` (Linux only).
- Docs/tasks/session_log edits happen only on the orchestration branch (`feat/claude-code-live-stream-json`), never from worktrees.
- Do not introduce tests that require a locally-installed `claude` binary. All gating must be fixture/synthetic and runnable on GitHub-hosted runners.
- Safety rule: do not emit raw backend lines in `agent_api` events; all parse errors must be redacted (see ADR-0010 + DRs).

## Branch & Worktree Conventions

- Orchestration branch: `feat/claude-code-live-stream-json`.
- Feature prefix: `ccsj`.
- Branch naming: `ccsj-<triad>-<scope>-<role>` (e.g., `ccsj-c0-stream-api-code`).
- Worktrees: `wt/<branch>` (in-repo; ignored by git).

## Triad Overview

- **C0 — `claude_code` streaming API:** Add a first-class streaming API to `crates/claude_code` for
  `--print --output-format stream-json`, yielding parsed events incrementally and supporting CRLF.
- **C1 — `agent_api` Claude live events:** Wire `agent_api` Claude backend to use the streaming API,
  emit events live, advertise `agent_api.events.live`, and preserve DR-0012 completion gating.

## CI Checkpoints

Bounded multi-OS validation runs only at explicit checkpoints:

- Checkpoint plan: `docs/project_management/next/claude-code-live-stream-json/ci_checkpoint_plan.md`
- Checkpoint task: `CP1-ci-checkpoint` (after `C1-integ`)
- Workflow (created as part of this feature): `.github/workflows/claude-code-live-stream-json-smoke.yml`

## Start Checklist (all tasks)

1. `git checkout feat/claude-code-live-stream-json && git pull --ff-only`
2. Read: this plan, `tasks.json`, `session_log.md`, the relevant spec, and your kickoff prompt.
3. Set the task status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add a START entry to `session_log.md`; commit docs (`docs: start <task-id>`).
5. Create the task branch and worktree from `feat/claude-code-live-stream-json`: `git worktree add -b <branch> wt/<branch> feat/claude-code-live-stream-json`.
6. Do **not** edit docs/tasks/session_log from the worktree.

## End Checklist (code/test)

1. Run required commands (code: fmt + clippy; test: fmt + targeted tests) and capture outputs.
2. From inside the worktree, commit task branch changes (no docs/tasks/session_log edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed). Do **not** merge into `feat/claude-code-live-stream-json`.
4. Checkout `feat/claude-code-live-stream-json`; update `tasks.json` status; add an END entry to `session_log.md` with commands/results/blockers; create downstream prompts if missing; commit docs (`docs: finish <task-id>`).
5. Remove the worktree: `git worktree remove wt/<branch>`.

## End Checklist (integration)

1. Merge code/test branches into the integration worktree; reconcile behavior to the spec.
2. Run (capture outputs):
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - Relevant tests (at minimum, suites introduced by the triad’s test task)
   - Linux-only gate: `make preflight`
3. Commit integration changes to the integration branch.
4. Fast-forward merge the integration branch into `feat/claude-code-live-stream-json`; update `tasks.json` and `session_log.md` with the END entry (commands/results/blockers); commit docs (`docs: finish <task-id>`).
5. Remove the worktree.

## Context Budget & Triad Sizing

- Keep each triad ≤ ~40–50% of a 272k token context window (~110–150k tokens).
- If the streaming API or protocol semantics expand significantly (more CLI surfaces, PTY, stderr mirroring),
  add more triads rather than growing C0/C1 beyond a single agent’s context.

