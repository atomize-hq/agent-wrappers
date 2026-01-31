# Kickoff - C1-integ (CODEX_WRAPPER_COVERAGE_AUTO_GENERATION)

You are the **integration agent** for C1 (merge C1 code+tests, reconcile to spec, run gates).

Scope source of truth: `docs/project_management/next/codex-wrapper-coverage-auto-generation/C1-spec.md`.

## Role boundaries (hard)
- You own reconciling implementation to spec and getting a clean, green result.
- Do not edit planning-pack docs (`docs/project_management/next/codex-wrapper-coverage-auto-generation/**`) from the worktree.
- Do not download binaries or run live Codex flows.
- Do not refresh committed artifacts (`cli_manifests/codex/wrapper_coverage.json` or reports) in C1 (deferred to C4).

## Start checklist
1. `git checkout feat/codex-wrapper-coverage-auto-generation && git pull --ff-only`
2. Read: `docs/project_management/next/codex-wrapper-coverage-auto-generation/plan.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/C1-spec.md`, this prompt.
3. Update `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`: set `C1-integ` to `in_progress`.
4. Add START entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: start C1-integ`.
5. Create worktree: `git worktree add -b wcg-c1-scenarios-3-6-integ wt/wcg-c1-scenarios-3-6-integ feat/codex-wrapper-coverage-auto-generation`
6. Work only inside `wt/wcg-c1-scenarios-3-6-integ` for integration changes.

## Integration requirements (C1)
- Merge branches:
  - `wcg-c1-scenarios-3-6-code`
  - `wcg-c1-scenarios-3-6-test`
- Reconcile to `C1-spec.md`.
- Sanity-check generation (do not commit outputs):
  - `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out /tmp/wrapper_coverage.json`

## Required commands (integration role)
- `cargo fmt`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Relevant tests (at minimum): `cargo test -p xtask`
- Integration gate: `make preflight`

## End checklist
1. Merge `wcg-c1-scenarios-3-6-code` and `wcg-c1-scenarios-3-6-test` into `wt/wcg-c1-scenarios-3-6-integ` and reconcile behavior to `C1-spec.md`.
2. Run required commands (capture outputs): `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p xtask`; `make preflight`.
3. Commit integration changes on branch `wcg-c1-scenarios-3-6-integ`.
4. Fast-forward merge `wcg-c1-scenarios-3-6-integ` into `feat/codex-wrapper-coverage-auto-generation`; set `C1-integ` to `completed` in `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`; add END entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: finish C1-integ`.
5. Remove worktree: `git worktree remove wt/wcg-c1-scenarios-3-6-integ`.

