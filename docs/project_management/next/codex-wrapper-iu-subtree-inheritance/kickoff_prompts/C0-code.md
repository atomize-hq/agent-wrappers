# C0 Code Kickoff — IU Subtree Inheritance (ADR 0004)

Scope: implement production changes only per `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C0-spec.md`.

Role boundaries:
- Production code only (no tests).
- Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`.

## Start Checklist
1. `git checkout feat/codex-wrapper-iu-subtree-inheritance && git pull --ff-only`
2. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/plan.md`.
3. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json`.
4. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`.
5. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C0-spec.md`.
6. Set task status (`C0-code`) to `in_progress` in `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json` (orchestration branch only).
7. Add START entry to `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`; commit docs (`docs: start C0-code`).
8. Create worktree: `git worktree add -b iu4-c0-report-iu-inheritance-code wt/iu4-c0-report-iu-inheritance-code feat/codex-wrapper-iu-subtree-inheritance`.
9. Do not edit docs/tasks/session_log from the worktree.

## Implementation Notes
- Update `crates/xtask/src/codex_report.rs` and `crates/xtask/src/codex_validate.rs` per C0 spec.
- Ensure RULES sorting fields added for this feature are parsed and validated (not silently ignored).

## End Checklist
1. Run: `cargo fmt`
2. Run: `cargo clippy --workspace --all-targets -- -D warnings`
3. Commit changes inside `wt/iu4-c0-report-iu-inheritance-code` (no docs/tasks/session_log edits).
4. Checkout `feat/codex-wrapper-iu-subtree-inheritance`; set `C0-code` to `completed`; add END entry; commit docs (`docs: finish C0-code`).
5. Remove worktree: `git worktree remove wt/iu4-c0-report-iu-inheritance-code`.

