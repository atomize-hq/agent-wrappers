# Kickoff - C3-code (CODEX_WRAPPER_COVERAGE_AUTO_GENERATION)

You are the **code agent** for C3 (production code only; no tests).

Scope source of truth: `docs/project_management/next/codex-wrapper-coverage-auto-generation/C3-spec.md`.

## Role boundaries (hard)
- Production code only; do not add/modify tests.
- Do not edit planning-pack docs (`docs/project_management/next/codex-wrapper-coverage-auto-generation/**`) from the worktree.
- Do not download binaries or run live Codex flows (no upstream `codex` execution).

## Start checklist
1. `git checkout feat/codex-wrapper-coverage-auto-generation && git pull --ff-only`
2. Read: `docs/project_management/next/codex-wrapper-coverage-auto-generation/plan.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`, `docs/project_management/next/codex-wrapper-coverage-auto-generation/C3-spec.md`, this prompt.
3. Update `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`: set `C3-code` to `in_progress`.
4. Add START entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: start C3-code`.
5. Create worktree: `git worktree add -b wcg-c3-scenarios-10-12-code wt/wcg-c3-scenarios-10-12-code feat/codex-wrapper-coverage-auto-generation`
6. Work only inside `wt/wcg-c3-scenarios-10-12-code` for code changes.

## Implementation requirements (C3)
- Extend `wrapper_coverage_manifest()` to implement Scenarios 10-12 exactly per `C3-spec.md`.
- Implement generation-time parity exclusions enforcement in `xtask codex-wrapper-coverage` exactly per `C3-spec.md`.

## Required commands (code role)
- `cargo fmt`
- `cargo clippy --workspace --all-targets -- -D warnings`

Allowed extra sanity check: `SOURCE_DATE_EPOCH=0 cargo run -p xtask -- codex-wrapper-coverage --out /tmp/wrapper_coverage.json`.

## End checklist
1. Run required commands and capture outputs: `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`.
2. Commit changes in `wt/wcg-c3-scenarios-10-12-code` (no planning-pack edits).
3. Outside the worktree, ensure branch `wcg-c3-scenarios-10-12-code` contains the commit (fast-forward if needed). Do not merge to `feat/codex-wrapper-coverage-auto-generation`.
4. Checkout `feat/codex-wrapper-coverage-auto-generation`; set `C3-code` to `completed` in `docs/project_management/next/codex-wrapper-coverage-auto-generation/tasks.json`; add END entry to `docs/project_management/next/codex-wrapper-coverage-auto-generation/session_log.md`; commit docs with `docs: finish C3-code`.
5. Remove worktree: `git worktree remove wt/wcg-c3-scenarios-10-12-code`.

