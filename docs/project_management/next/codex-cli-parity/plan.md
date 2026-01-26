# CODEX_CLI_PARITY – Plan

Source: `docs/adr/0001-codex-cli-parity-maintenance.md`

## Purpose
Implement the ADR 0001 “CLI Snapshot → Diff → Update” release-trailing workflow so this repo can detect Codex CLI drift, validate against real binaries, and keep wrapper behavior and JSONL parsing compatible across supported versions.

## Guardrails
- Triads only: code / test / integration. No mixed roles.
- Specs are the source of truth; integration reconciles code/tests to the spec.
- Code: production (non-test) changes only. Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`.
- Test: tests/fixtures/harnesses only. Required commands: `cargo fmt`; targeted `cargo test ...` for suites added/touched.
- Integration: merges code+tests, reconciles to spec, and must run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, relevant tests, and `make preflight`.
- Planning-pack docs edits happen only on the orchestration branch (`feat/codex-cli-parity`), never from worktrees:
  - `docs/project_management/next/codex-cli-parity/tasks.json`
  - `docs/project_management/next/codex-cli-parity/session_log.md`
- Safety: this feature must not introduce crate-runtime binary auto-download/auto-update behavior; any downloads happen only in CI/workflows per ADR.

## Branch & Worktree Conventions
- Orchestration branch: `feat/codex-cli-parity`.
- Branch naming pattern: `ccp-<triad>-<scope>-<role>`.
- Branches used by this feature (exact):
  - C0: `ccp-c0-snapshot-code`, `ccp-c0-snapshot-test`, `ccp-c0-snapshot-integ`
  - C1: `ccp-c1-validation-code`, `ccp-c1-validation-test`, `ccp-c1-validation-integ`
  - C2: `ccp-c2-jsonl-code`, `ccp-c2-jsonl-test`, `ccp-c2-jsonl-integ`
  - C3: `ccp-c3-ops-code`, `ccp-c3-ops-test`, `ccp-c3-ops-integ`
- Worktrees: `wt/<branch>` (in-repo; ignored by git).

## Triad Overview
- **C0 – Snapshot schema + generator:** Add `crates/xtask` with `xtask codex-snapshot ...` and define the canonical on-disk snapshot layout under `cli_manifests/codex/` including `current.json`, `raw_help/<version>/...`, and `supplement/commands.json`.
- **C1 – Version policy + CI workflows:** Create `.github/workflows/ci.yml`, `.github/workflows/codex-cli-release-watch.yml`, `.github/workflows/codex-cli-update-snapshot.yml`, and `cli_manifests/codex/artifacts.lock.json` to validate real binaries on Linux and automate snapshot updates (downloads only in CI/workflows; upstream tags look like `rust-v<version>` and the Linux musl asset is `codex-x86_64-unknown-linux-musl.tar.gz`).
- **C2 – JSONL + notifications compatibility:** Add drift-tolerant parsing/normalization plus fixtures at `crates/codex/examples/fixtures/versioned/` and tests in `crates/codex/tests/jsonl_compat.rs`.
- **C3 – Ops playbook + promotion rules:** Write the maintainer runbook at `cli_manifests/codex/OPS_PLAYBOOK.md` (linked from `cli_manifests/codex/README.md`) including the trial-run checklist and promotion criteria for intentionally unwrapped surfaces.

## Start Checklist (all tasks)
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: this plan, `tasks.json`, `session_log.md`, the relevant `C*-spec.md`, and your kickoff prompt.
3. Set the task status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add a START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start <task-id>`).
5. Create the task branch and worktree from `feat/codex-cli-parity`: `git worktree add -b <branch> wt/<branch> feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## End Checklist (code/test)
1. Run required commands (code: fmt + clippy; test: fmt + targeted tests) and capture outputs.
2. From inside the worktree, commit task branch changes (no planning-pack docs edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed). Do **not** merge into `feat/codex-cli-parity`.
4. Checkout `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` status; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish <task-id>`).
5. Remove the worktree: `git worktree remove wt/<branch>`.

## End Checklist (integration)
1. Merge code/test branches into the integration worktree; reconcile behavior to the spec.
2. Run (capture outputs):
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - Relevant tests (at minimum, the suites introduced by the triad’s test task)
   - Integration gate: `make preflight`
3. Commit integration changes to the integration branch.
4. Fast-forward merge the integration branch into `feat/codex-cli-parity`; update `tasks.json` and `session_log.md` with the END entry; commit docs (`docs: finish <task-id>`).
5. Remove the worktree.

## Context Budget & Triad Sizing
- Aim for each triad to fit comfortably within ≤ ~40–50% of a 272k context window (spec + code/tests + recent history).
- If a triad starts expanding (multiple platforms, many commands, broad refactors), split into additional `C<N>` phases before kickoff.
