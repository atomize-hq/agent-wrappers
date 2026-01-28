# CODEX_CLI_PARITY_COVERAGE_MAPPING – Plan

Source: `docs/adr/0002-codex-cli-parity-coverage-mapping.md`

## Purpose
Implement the ADR 0002 “Snapshot → Coverage → Work Queue” system so maintainers can deterministically:
- generate per-target upstream snapshots (with best-effort feature enabling),
- merge into a union snapshot with conflict recording,
- generate a deterministic `wrapper_coverage.json` from wrapper code,
- produce deterministic coverage reports and version metadata,
- validate committed artifacts against `cli_manifests/codex/SCHEMA.json` says + `cli_manifests/codex/RULES.json`,
- and wire CI so new upstream stable releases yield an actionable, granular work queue (commands + flags + positional args).

## Guardrails
- Triads only: code / test / integration. No mixed roles.
- Specs are the source of truth; integration reconciles code/tests to the spec.
- Code: production code only; no tests. Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`.
- Test: tests/fixtures/harnesses only; no production logic. Required commands: `cargo fmt`; targeted `cargo test ...` for suites added/touched.
- Integration: merges code+tests, reconciles to spec, and must run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, relevant tests, and `make preflight`.
- Planning-pack docs edits happen only on the orchestration branch (`feat/codex-cli-parity-coverage-mapping`), never from worktrees:
  - `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`
  - `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`
- Safety: this feature must not introduce crate-runtime binary auto-download/auto-update behavior; any upstream release discovery/download remains CI/workflow-driven (per ADR 0001).

## Branch & Worktree Conventions
- Orchestration branch: `feat/codex-cli-parity-coverage-mapping`.
- Branch naming pattern: `ccm-<triad>-<scope>-<role>`.
- Worktrees: `wt/<branch>` (in-repo; ignored by git).

## Triad Overview
- **C0 – Deterministic validators:** Add `xtask codex-validate` (normative CLI in `C0-spec.md`) to enforce `SCHEMA.json` + `RULES.json` + `VALIDATOR_SPEC.md` invariants for committed manifests (pointers, versions metadata, snapshots, reports, wrapper coverage).
- **C1 – Per-target snapshots + union builder:** Extend `xtask codex-snapshot` (add `--out-file` + `--raw-help-target`) to write per-target snapshots under `cli_manifests/codex/snapshots/<version>/<target_triple>.json`, and implement `xtask codex-union` to produce `snapshots/<version>/union.json` with conflict recording.
- **C2 – Wrapper coverage generator:** Implement `xtask codex-wrapper-coverage` to generate `cli_manifests/codex/wrapper_coverage.json` from the single source of truth in `crates/codex/src/wrapper_coverage_manifest.rs` (not hand-edited).
- **C3 – Coverage reports + version metadata:** Implement `xtask codex-report`, `xtask codex-version-metadata`, and `xtask codex-retain` per `RULES.json` (reports under `reports/<version>/coverage.*.json`, metadata under `versions/<version>.json`, mechanical retention).
- **C4 – CI wiring (end-to-end):** Extend `.github/workflows/codex-cli-update-snapshot.yml` to run the multi-platform snapshot→union→wrapper-coverage→report→validate pipeline and upload artifacts; add a hard-gate `xtask codex-validate` job in `ci.yml` when committed artifacts exist; preserve the “PR best-effort + artifact fallback” policy.

## Start Checklist (all tasks)
1. `git checkout feat/codex-cli-parity-coverage-mapping && git pull --ff-only`
2. Read: this plan, `tasks.json`, `session_log.md`, the relevant `C*-spec.md`, and your kickoff prompt.
3. Set the task status to `in_progress` in `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` (orchestration branch only).
4. Add a START entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`; commit docs (`docs: start <task-id>`).
5. Create the task branch and worktree from `feat/codex-cli-parity-coverage-mapping`: `git worktree add -b <branch> wt/<branch> feat/codex-cli-parity-coverage-mapping`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` or `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` from the worktree.

## End Checklist (code/test)
1. Run required commands (code: fmt + clippy; test: fmt + targeted tests) and capture outputs.
2. From inside the worktree, commit task branch changes (no planning-pack docs edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed). Do **not** merge into `feat/codex-cli-parity-coverage-mapping`.
4. Checkout `feat/codex-cli-parity-coverage-mapping`; update `tasks.json` status; add an END entry to `session_log.md` with commands/results/blockers; commit docs (`docs: finish <task-id>`).
5. Remove the worktree: `git worktree remove wt/<branch>`.

## End Checklist (integration)
1. Merge code/test branches into the integration worktree; reconcile behavior to the spec.
2. Run (capture outputs):
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - Relevant tests (at minimum, the suites introduced by the triad’s test task)
   - Integration gate: `make preflight`
3. Commit integration changes to the integration branch.
4. Fast-forward merge the integration branch into `feat/codex-cli-parity-coverage-mapping`; update `tasks.json` and `session_log.md` with the END entry; commit docs (`docs: finish <task-id>`).
5. Remove the worktree.

## Context Budget & Triad Sizing
- Aim for each triad to fit comfortably within ≤ ~40–50% of a 272k context window (spec + code/tests + recent history).
- If a triad starts expanding (platform matrices, schema churn, broad refactors), split into additional `C<N>` phases before kickoff.
