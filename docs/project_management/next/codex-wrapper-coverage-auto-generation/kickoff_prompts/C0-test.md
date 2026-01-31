# Kickoff - C0-test (CODEX_WRAPPER_COVERAGE_AUTO_GENERATION)

You are the **test agent** for C0 (tests/fixtures only; no production code).

Scope source of truth: `docs/project_management/next/codex-wrapper-coverage-auto-generation/C0-spec.md`.

## Role boundaries (hard)
- Tests/fixtures/harnesses only; do not change production code.
- Do not edit planning-pack docs (`docs/project_management/next/codex-wrapper-coverage-auto-generation/**`) from the worktree.
- Do not download binaries or run live Codex flows.

## Start checklist
1. `git checkout feat/codex-wrapper-coverage-auto-generation && git pull --ff-only`
2. Read: `docs/project_management/next/codex-wrapper-coverage-auto-generation/plan.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/C0-spec.md`, this prompt.
3. Update `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`: set `C0-test` to `in_progress`.
4. Add START entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: start C0-test`.
5. Create worktree: `git worktree add -b wcg-c0-manifest-core-test wt/wcg-c0-manifest-core-test feat/codex-wrapper-coverage-auto-generation`
6. Work only inside `wt/wcg-c0-manifest-core-test` for test changes.

## Test requirements (C0)
Add/update tests under `crates/xtask/tests/` to cover:
- `SOURCE_DATE_EPOCH` is required for `xtask codex-wrapper-coverage` (missing/invalid fails deterministically).
- Output is deterministic and non-empty when `SOURCE_DATE_EPOCH` is set.
- v1 scope omission and note restrictions are enforced.

## Required commands (test role)
- `cargo fmt`
- Targeted tests for what you add/touch (at minimum): `cargo test -p xtask`

## End checklist
1. Run required commands and capture outputs: `cargo fmt`; `cargo test -p xtask` (and any additional targeted tests for files touched).
2. Commit changes in `wt/wcg-c0-manifest-core-test` (no planning-pack edits).
3. Outside the worktree, ensure branch `wcg-c0-manifest-core-test` contains the commit (fast-forward if needed). Do not merge to `feat/codex-wrapper-coverage-auto-generation`.
4. Checkout `feat/codex-wrapper-coverage-auto-generation`; set `C0-test` to `completed` in `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`; add END entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: finish C0-test`.
5. Remove worktree: `git worktree remove wt/wcg-c0-manifest-core-test`.
