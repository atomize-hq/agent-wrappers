# Kickoff - C2-integ (CODEX_WRAPPER_COVERAGE_AUTO_GENERATION)

You are the **integration agent** for C2 (merge C2 code+tests, reconcile to spec, run gates).

Scope source of truth: `docs/project_management/next/codex-wrapper-coverage-auto-generation/C2-spec.md`.

## Role boundaries (hard)
- You own reconciling implementation to spec and getting a clean, green result.
- Do not edit planning-pack docs (`docs/project_management/next/codex-wrapper-coverage-auto-generation/**`) from the worktree.
- Do not download binaries or run live Codex flows.
- Do not refresh committed artifacts (`cli_manifests/codex/wrapper_coverage.json` or reports) in C2 (deferred to C4).

## Start checklist
1. `git checkout feat/codex-wrapper-coverage-auto-generation && git pull --ff-only`
2. Read: `docs/project_management/next/codex-wrapper-coverage-auto-generation/plan.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/C2-spec.md`, this prompt.
3. Update `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`: set `C2-integ` to `in_progress`.
4. Add START entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: start C2-integ`.
5. Create worktree: `git worktree add -b wcg-c2-scenarios-7-9-integ wt/wcg-c2-scenarios-7-9-integ feat/codex-wrapper-coverage-auto-generation`
6. Work only inside `wt/wcg-c2-scenarios-7-9-integ` for integration changes.

## Integration requirements (C2)
- Merge branches:
  - `wcg-c2-scenarios-7-9-code`
  - `wcg-c2-scenarios-7-9-test`
- Reconcile to `C2-spec.md`.
- Sanity-check generation (do not commit outputs):
  - `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out /tmp/wrapper_coverage.json`

## Required commands (integration role)
- `cargo fmt`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Relevant tests (at minimum): `cargo test -p xtask`
- Integration gate: `make preflight`

## End checklist
1. Merge `wcg-c2-scenarios-7-9-code` and `wcg-c2-scenarios-7-9-test` into `wt/wcg-c2-scenarios-7-9-integ` and reconcile behavior to `C2-spec.md`.
2. Run required commands (capture outputs): `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p xtask`; `make preflight`.
3. Commit integration changes on branch `wcg-c2-scenarios-7-9-integ`.
4. Fast-forward merge `wcg-c2-scenarios-7-9-integ` into `feat/codex-wrapper-coverage-auto-generation`; set `C2-integ` to `completed` in `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`; add END entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: finish C2-integ`.
5. Remove worktree: `git worktree remove wt/wcg-c2-scenarios-7-9-integ`.

