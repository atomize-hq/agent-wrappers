# Kickoff - C0-integ (CODEX_WRAPPER_COVERAGE_AUTO_GENERATION)

You are the **integration agent** for C0 (merge code+tests, reconcile to spec, run gates).

Scope source of truth: `docs/project_management/next/codex-wrapper-coverage-auto-generation/C0-spec.md`.

## Role boundaries (hard)
- You own reconciling implementation to spec and getting a clean, green result.
- Do not edit planning-pack docs (`docs/project_management/next/codex-wrapper-coverage-auto-generation/**`) from the worktree.
- Do not download binaries or run live Codex flows.

## Start checklist
1. `git checkout feat/codex-wrapper-coverage-auto-generation && git pull --ff-only`
2. Read: `docs/project_management/next/codex-wrapper-coverage-auto-generation/plan.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/C0-spec.md`, this prompt.
3. Update `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`: set `C0-integ` to `in_progress`.
4. Add START entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: start C0-integ`.
5. Create worktree: `git worktree add -b wcg-c0-manifest-core-integ wt/wcg-c0-manifest-core-integ feat/codex-wrapper-coverage-auto-generation`
6. Work only inside `wt/wcg-c0-manifest-core-integ` for integration changes.

## Integration requirements (C0)
- Merge branches:
  - `wcg-c0-manifest-core-code`
  - `wcg-c0-manifest-core-test`
- Reconcile to `C0-spec.md`.
- Generate a local wrapper coverage file to verify behavior:
  - `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out /tmp/wrapper_coverage.json`
- Do not modify the committed `cli_manifests/codex/wrapper_coverage.json` artifact in C0; C4 integration owns the first committed refresh for ADR 0003.

## Required commands (integration role)
- `cargo fmt`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Relevant tests (at minimum): `cargo test -p xtask`
- Integration gate: `make preflight`

## End checklist
1. Merge `wcg-c0-manifest-core-code` and `wcg-c0-manifest-core-test` into `wt/wcg-c0-manifest-core-integ` and reconcile behavior to `C0-spec.md`.
2. Run required commands (capture outputs): `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p xtask`; `make preflight`.
3. Commit integration changes on branch `wcg-c0-manifest-core-integ`.
4. Fast-forward merge `wcg-c0-manifest-core-integ` into `feat/codex-wrapper-coverage-auto-generation`; set `C0-integ` to `completed` in `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`; add END entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: finish C0-integ`.
5. Remove worktree: `git worktree remove wt/wcg-c0-manifest-core-integ`.
