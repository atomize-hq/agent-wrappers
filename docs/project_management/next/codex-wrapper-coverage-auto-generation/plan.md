# CODEX_WRAPPER_COVERAGE_AUTO_GENERATION - Plan

Source: `docs/adr/0003-wrapper-coverage-auto-generation.md`

Normative specs:
- `docs/specs/codex-wrapper-coverage-generator-contract.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md`

## Purpose
Implement ADR 0003 by producing a **non-empty, deterministic** `cli_manifests/codex/wrapper_coverage.json` derived from `crates/codex` implementation signals, so parity reports become meaningful deltas instead of "everything missing".

## Guardrails
- Docs/planning are done in this pack only; execution happens later on `feat/codex-wrapper-coverage-auto-generation`.
- Triads only: code / test / integration. No mixed roles.
- Code: production code only; no tests. Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`.
- Test: tests/fixtures/harnesses only; no production logic. Required commands: `cargo fmt`; `cargo test -p xtask` (and any additional targeted tests for files touched).
- Integration: merges code+tests, reconciles to spec, and must run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, relevant tests, and `make preflight`.
- Planning-pack edits (this directory) happen only on the orchestration branch (`feat/codex-wrapper-coverage-auto-generation`), never from worktrees.

## Determinism & Policy (must not drift)
- `xtask codex-wrapper-coverage` MUST require `SOURCE_DATE_EPOCH` and fail if missing/invalid.
- Wrapper-derived coverage MUST be deterministic and offline:
  - no network access
  - no filesystem reads for discovery
  - no subprocess execution (do not run a Codex binary)
  - no wall-clock time and no randomness
- v1 scope rule: no emitted scope fields anywhere (`scope` MUST be omitted for commands/flags/args).
- v1 note rule: `note` is restricted to:
  - `intentionally_unsupported` rationale notes (non-empty, validator-enforced)
  - `capability-guarded` (exact string) for capability-guarded surfaces
  - otherwise omit `note`
- Parity exclusions (TUI policy):
  - `cli_manifests/codex/RULES.json.parity_exclusions` defines excluded identities.
  - Reports MUST classify excluded identities under `excluded_*` deltas (not `missing_*`).
  - The wrapper coverage generator MUST NOT emit excluded identities (validator-enforced; generation-time rejection is implemented in C3).

## Branch & Worktree Conventions
- Orchestration branch: `feat/codex-wrapper-coverage-auto-generation`.
- Branch naming pattern: `wcg-<triad>-<scope>-<role>`.
- Worktrees: `wt/<branch>` (in-repo; ignored by git).

## Triad Overview
- **C0 - Non-empty deterministic core:** Implement the wrapper-derived manifest skeleton and `xtask codex-wrapper-coverage` determinism enforcement so `wrapper_coverage.json` can be generated offline and non-empty (seed Scenario 0-2; establish note/scope rules).
- **C1 - Scenario Catalog v1 (3-6):** Implement Scenarios 3-6 and add tests that lock those scenarios down.
- **C2 - Scenario Catalog v1 (7-9):** Implement Scenarios 7-9 and add tests that lock those scenarios down.
- **C3 - Scenario Catalog v1 (10-12):** Implement Scenarios 10-12, implement parity exclusions rejection at generation time, and add tests that lock those behaviors down.
- **C4 - Integration + docs:** Merge all scenario slices, refresh the committed `cli_manifests/codex/wrapper_coverage.json`, run report/validate gates, and reconcile documentation if required.

## Start Checklist (all tasks)
1. `git checkout feat/codex-wrapper-coverage-auto-generation && git pull --ff-only`
2. Read: this plan, `tasks.json`, `session_log.md`, the relevant `C*-spec.md`, and your kickoff prompt.
3. Set the task status to `in_progress` in `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json` (orchestration branch only).
4. Add a START entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs (`docs: start <task-id>`).
5. Create the task branch and worktree from `feat/codex-wrapper-coverage-auto-generation`: `git worktree add -b <branch> wt/<branch> feat/codex-wrapper-coverage-auto-generation`.
6. Do **not** edit `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json` or `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md` from the worktree.

## End Checklist (code/test)
1. Run required commands (code: fmt + clippy; test: fmt + targeted tests) and capture outputs.
2. From inside the worktree, commit task branch changes (no planning-pack edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed). Do **not** merge into `feat/codex-wrapper-coverage-auto-generation`.
4. Checkout `feat/codex-wrapper-coverage-auto-generation`; update `tasks.json` status; add an END entry to `session_log.md` with commands/results/blockers; commit docs (`docs: finish <task-id>`).
5. Remove the worktree: `git worktree remove wt/<branch>`.

## End Checklist (integration)
1. Merge code/test branches into the integration worktree; reconcile behavior to the spec.
2. Run (capture outputs):
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - Relevant tests (at minimum, the suites introduced by the triad's test task)
   - Integration gate: `make preflight`
3. Commit integration changes to the integration branch.
4. Fast-forward merge the integration branch into `feat/codex-wrapper-coverage-auto-generation`; update `tasks.json` and `session_log.md` with the END entry; commit docs (`docs: finish <task-id>`).
5. Remove the worktree.

## Context Budget & Triad Sizing
- Keep each triad small enough that a single agent can hold the spec + surrounding code comfortably (<= ~40-50% of a 272k context window).
- If scenario coverage refactors start to sprawl, split into additional `C<N>` phases before kickoff.
