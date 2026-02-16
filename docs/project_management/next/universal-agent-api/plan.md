# Universal Agent API — Plan

## Purpose

Create a new universal Rust API (`agent_api`) that can target multiple CLI agent backends (starting
with Codex and Claude Code) behind a unified run + events contract with explicit capability
discovery. The goal is to keep downstream orchestration code stable as we add more agents.

## Guardrails

- Triads only: code / test / integration. No mixed roles.
- Code: production code only; no tests. Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Test: tests/fixtures/harnesses only; no production logic. Required commands: `cargo fmt`; targeted `cargo test ...` for suites added/touched.
- Integration: merges code+tests, reconciles to spec, and must run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, relevant tests, and the repo gate: `make preflight` (Linux only).
- Docs/tasks/session_log edits happen only on the orchestration branch (`feat/universal-agent-api`), never from worktrees.
- Do not introduce tests that require locally-installed agent binaries (Codex/Claude). Gating must be fixture/sample-based and run on GitHub-hosted runners.

## Branch & Worktree Conventions

- Orchestration branch: `feat/universal-agent-api`.
- Feature prefix: `uaa`.
- Branch naming: `uaa-<triad>-<scope>-<role>` (e.g., `uaa-c0-core-code`).
- Worktrees: `wt/<branch>` (in-repo; ignored by git).

## Triad Overview

- **C0 — Core universal API crate:** Introduce `crates/agent_api` with the open-set agent identity,
  unified event envelope, capability model, backend registry/gateway, and error taxonomy.
- **C1 — Codex backend adapter:** Add a feature-gated Codex backend implemented via `crates/codex`,
  mapping Codex streaming events into the universal event envelope.
- **C2 — Claude Code backend adapter:** Add a feature-gated Claude Code backend implemented via
  `crates/claude_code`, producing the universal event envelope via buffered/stream-json parsing
  (per the run protocol + capability model).

## CI Checkpoints

Bounded multi-OS validation runs only at explicit checkpoints:

- Checkpoint plan: `docs/project_management/next/universal-agent-api/ci_checkpoint_plan.md`
- Checkpoint task: `CP1-ci-checkpoint` (after `C2-integ`)

## Start Checklist (all tasks)

1. `git checkout feat/universal-agent-api && git pull --ff-only`
2. Read: this plan, `tasks.json`, `session_log.md`, the relevant spec, and your kickoff prompt.
3. Set the task status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add a START entry to `session_log.md`; commit docs (`docs: start <task-id>`).
5. Create the task branch and worktree from `feat/universal-agent-api`: `git worktree add -b <branch> wt/<branch> feat/universal-agent-api`.
6. Do **not** edit docs/tasks/session_log from the worktree.

## End Checklist (code/test)

1. Run required commands (code: fmt + clippy; test: fmt + targeted tests) and capture outputs.
2. From inside the worktree, commit task branch changes (no docs/tasks/session_log edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed). Do **not** merge into `feat/universal-agent-api`.
4. Checkout `feat/universal-agent-api`; update `tasks.json` status; add an END entry to `session_log.md` with commands/results/blockers; create downstream prompts if missing; commit docs (`docs: finish <task-id>`).
5. Remove the worktree: `git worktree remove wt/<branch>`.

## End Checklist (integration)

1. Merge code/test branches into the integration worktree; reconcile behavior to the spec.
2. Run (capture outputs):
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   - Relevant tests (at minimum, suites introduced by the triad’s test task)
   - Linux-only gate: `make preflight`
3. Commit integration changes to the integration branch.
4. Fast-forward merge the integration branch into `feat/universal-agent-api`; update `tasks.json` and `session_log.md` with the END entry (commands/results/blockers); commit docs (`docs: finish <task-id>`).
5. Remove the worktree.

## Context Budget & Triad Sizing

- Keep each triad ≤ ~40–50% of a 272k token context window (~110–150k tokens).
- If the universal API expands beyond C0–C2 (more backends or new protocol surfaces), add more triads
  rather than growing a single slice.

