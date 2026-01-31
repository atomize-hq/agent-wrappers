# Codex Wrapper – IU Subtree Inheritance (ADR 0004) – Plan

## Purpose
Implement deterministic report-time “intentionally_unsupported (IU) subtree inheritance” (ADR 0004) so intentionally unwrapped command families stop producing noisy `missing_*` deltas, while remaining audit-visible under `deltas.intentionally_unsupported`.

## Guardrails
- Triads only: code / test / integration. No mixed roles.
- Code: production code only; no tests. Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`.
- Test: tests/fixtures only; no production logic. Required commands: `cargo fmt` plus the exact `cargo test ...` command(s) specified by the triad’s kickoff prompt (no substitutions).
- Integration: merges code+tests, reconciles to spec, and runs the exact command set specified by the triad’s kickoff prompt (no substitutions).
- Docs/tasks/session_log edits happen only on the orchestration branch (`feat/codex-wrapper-iu-subtree-inheritance`), never from worktrees.
- Do not change CI workflows as part of this feature.

## Prerequisites (must be true before starting any task)
- Wrapper coverage auto-generation is already implemented and produces a non-empty `cli_manifests/codex/wrapper_coverage.json` (see `docs/adr/0003-wrapper-coverage-auto-generation.md`).
- An upstream union snapshot exists under `cli_manifests/codex/snapshots/<version>/union.json`.
- `jq` is available in the execution environment (used for deterministic inspection in integration checklists).

## Branch & Worktree Conventions
- Orchestration branch: `feat/codex-wrapper-iu-subtree-inheritance`.
- Feature prefix: `iu4`.
- Branch naming: use the exact branch names specified in `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json` (do not invent new names).
- Worktrees: `wt/<branch>` (in-repo; ignored by git).

## Triad Overview
- **C0 – Report + validate IU inheritance:** Implement ADR 0004 classification in `xtask codex-report`, wire validator invariants, and add tests to lock behavior down.
- **C1 – Adopt IU roots:** Add IU subtree roots for intentionally unwrapped command families in wrapper coverage source-of-truth and validate resulting report deltas.

## Start Checklist (all tasks)
1. `git checkout feat/codex-wrapper-iu-subtree-inheritance && git pull --ff-only`
2. Read: this plan, `tasks.json`, `session_log.md`, the relevant spec, and your kickoff prompt.
3. Set the task status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add a START entry to `session_log.md`; commit docs with message `docs: start <TASK_ID>` where `<TASK_ID>` is exactly the `id` field from `tasks.json` (example: `docs: start C0-code`).
5. Create the task branch and worktree using the exact branch/worktree values from `tasks.json`.
6. Do **not** edit docs/tasks/session_log from the worktree.

## End Checklist (code/test)
1. Run required commands (code: fmt + clippy; test: fmt + targeted tests) and capture outputs.
2. From inside the worktree, commit task branch changes (no docs/tasks/session_log edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed). Do **not** merge into `feat/codex-wrapper-iu-subtree-inheritance`.
4. Checkout `feat/codex-wrapper-iu-subtree-inheritance`; update `tasks.json` status; add an END entry to `session_log.md` with commands/results/blockers; commit docs with message `docs: finish <TASK_ID>` where `<TASK_ID>` is exactly the `id` field from `tasks.json` (example: `docs: finish C0-code`).
5. Remove the worktree: `git worktree remove wt/<branch>`.

## End Checklist (integration)
1. Merge code/test branches into the integration worktree; reconcile behavior to the spec.
2. Set a deterministic timestamp for regenerated artifacts: `export SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)"`
3. Run the exact command set specified by the triad’s integration kickoff prompt (capture outputs in the session log).
4. Commit integration changes to the integration branch.
5. Fast-forward merge the integration branch into `feat/codex-wrapper-iu-subtree-inheritance`; update `tasks.json` and `session_log.md` with the END entry (commands/results/blockers); commit docs with message `docs: finish <TASK_ID>` where `<TASK_ID>` is exactly the `id` field from `tasks.json` (example: `docs: finish C0-integ`).
6. Remove the worktree.
