# claude-code-cli-parity-2.1.32 – Plan – Plan

## Purpose
<One paragraph describing what this feature/sprint accomplishes and why.>

## Guardrails
- Triads only: code / test / integration. No mixed roles.
- Code: production code only; no tests. Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`. Optional targeted sanity checks allowed.
- Test: tests/fixtures/harnesses only; no production logic. Required commands: `cargo fmt`; targeted `cargo test ...` for suites added/touched.
- Integration: merges code+tests, reconciles to spec, and must run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, relevant tests, and the repo’s integration gate (see “End Checklist (integration)”).
- Docs/tasks/session_log edits happen only on the orchestration branch (`feat/claude-code-cli-parity-2.1.32`), never from worktrees.
- Respect protected paths when doing filesystem work (see the triad spec(s)); never touch `.git`, `.substrate-git`, `.substrate`, sockets, or device nodes unless explicitly in-scope.

## Branch & Worktree Conventions
- Orchestration branch: `feat/claude-code-cli-parity-2.1.32`.
- Branch naming: `claude-code-cli-parity-2-1-32-<triad>-v2-1-32-<role>` (e.g., `k-c0-schema-code`).
- Worktrees: `wt/<branch>` (in-repo; ignored by git).

## Triad Overview
- **C0 – <Title>:** <1–2 sentences.>
- **C1 – <Title>:** <1–2 sentences.>
- **C2 – <Title>:** <1–2 sentences.>

## Start Checklist (all tasks)
1. `git checkout feat/claude-code-cli-parity-2.1.32 && git pull --ff-only`
2. Read: this plan, `tasks.json`, `session_log.md`, the relevant spec, and your kickoff prompt.
3. Set the task status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add a START entry to `session_log.md`; commit docs (`docs: start <task-id>`).
5. Create the task branch and worktree from `feat/claude-code-cli-parity-2.1.32`: `git worktree add -b <branch> wt/<branch> feat/claude-code-cli-parity-2.1.32`.
6. Do **not** edit docs/tasks/session_log from the worktree.

## End Checklist (code/test)
1. Run required commands (code: fmt + clippy; test: fmt + targeted tests) and capture outputs.
2. From inside the worktree, commit task branch changes (no docs/tasks/session_log edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed). Do **not** merge into `feat/claude-code-cli-parity-2.1.32`.
4. Checkout `feat/claude-code-cli-parity-2.1.32`; update `tasks.json` status; add an END entry to `session_log.md` with commands/results/blockers; create downstream prompts if missing; commit docs (`docs: finish <task-id>`).
5. Remove the worktree: `git worktree remove wt/<branch>`.

## End Checklist (integration)
1. Merge code/test branches into the integration worktree; reconcile behavior to the spec.
2. Run (capture outputs):
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - Relevant tests (at minimum, the suites introduced by the triad’s test task)
   - Integration gate: `make preflight`
3. Commit integration changes to the integration branch.
4. Fast-forward merge the integration branch into `feat/claude-code-cli-parity-2.1.32`; update `tasks.json` and `session_log.md` with the END entry (commands/results/blockers); commit docs (`docs: finish <task-id>`).
5. Remove the worktree.

## Context Budget & Triad Sizing
- Agents typically have a 272k token context window. Size each triad so a single agent needs no more than ~40–50% of that window (roughly 110–150k tokens) to hold the spec, plan, code/tests, and recent history.
- If a triad risks breaching that budget (large migration, many platforms, broad refactors), split into additional triads or narrower phases before kickoff.


## Parity Inputs
- Parity root: `cli_manifests/claude_code`
- Version: `2.1.32`
- Coverage report: `cli_manifests/claude_code/reports/2.1.32/coverage.any.json`
