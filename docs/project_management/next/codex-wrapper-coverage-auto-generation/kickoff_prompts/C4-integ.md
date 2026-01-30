# Kickoff - C4-integ (CODEX_WRAPPER_COVERAGE_AUTO_GENERATION)

You are the **integration agent** for C4 (merge all code+tests, reconcile to specs, refresh artifacts, run gates, and reconcile docs if required).

Scope source of truth: `docs/project_management/next/codex-wrapper-coverage-auto-generation/C4-spec.md`.

## Role boundaries (hard)
- You own reconciling implementation to spec and getting a clean, green result.
- Do not edit planning-pack docs (`docs/project_management/next/codex-wrapper-coverage-auto-generation/**`) from the worktree.
- Do not download binaries or run live Codex flows.

## Start checklist
1. `git checkout feat/codex-wrapper-coverage-auto-generation && git pull --ff-only`
2. Read: `docs/project_management/next/codex-wrapper-coverage-auto-generation/plan.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/C4-spec.md`, this prompt.
3. Update `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`: set `C4-integ` to `in_progress`.
4. Add START entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: start C4-integ`.
5. Create worktree: `git worktree add -b wcg-c4-integration wt/wcg-c4-integration feat/codex-wrapper-coverage-auto-generation`
6. Work only inside `wt/wcg-c4-integration` for integration changes.

## Integration requirements (C4)
- Merge branches:
  - `wcg-c1-scenarios-3-6-code`
  - `wcg-c1-scenarios-3-6-test`
  - `wcg-c2-scenarios-7-9-code`
  - `wcg-c2-scenarios-7-9-test`
  - `wcg-c3-scenarios-10-12-code`
  - `wcg-c3-scenarios-10-12-test`
- Reconcile to `C1-spec.md`, `C2-spec.md`, `C3-spec.md`, `C4-spec.md`.
- Refresh committed wrapper coverage artifact:
  - `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out cli_manifests/codex/wrapper_coverage.json`
- Run report + validate checks against the existing committed snapshots:
  - `VERSION="$(tr -d '\\n' < cli_manifests/codex/latest_validated.txt)"`
  - `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-report --version "$VERSION" --root cli_manifests/codex`
  - `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-validate --root cli_manifests/codex`
- If implementation creates contradictions with ADR/spec docs, update those docs to eliminate contradictions.

## Required commands (integration role)
- `cargo fmt`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Relevant tests (at minimum): `cargo test -p xtask`
- Integration gate: `make preflight`

## End checklist
1. Merge all upstream branches into `wt/wcg-c4-integration` and reconcile behavior to all specs.
2. Run required commands (capture outputs): `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p xtask`; `make preflight`.
3. Additionally run and capture outputs: `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out cli_manifests/codex/wrapper_coverage.json`; `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-report --version \"$(cat cli_manifests/codex/latest_validated.txt | tr -d '\\n')\" --root cli_manifests/codex`; `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-validate --root cli_manifests/codex`.
4. Commit integration changes on branch `wcg-c4-integration`.
5. Fast-forward merge `wcg-c4-integration` into `feat/codex-wrapper-coverage-auto-generation`; set `C4-integ` to `completed` in `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`; add END entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: finish C4-integ`.
6. Remove worktree: `git worktree remove wt/wcg-c4-integration`.

