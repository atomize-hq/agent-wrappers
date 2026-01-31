# C1 Code Kickoff — Adopt IU Roots

Scope: implement production changes only per `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C1-spec.md`.

Role boundaries:
- Production code only (no tests).
- Required commands: `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`.

## Start Checklist
1. `git checkout feat/codex-wrapper-iu-subtree-inheritance && git pull --ff-only`
2. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/plan.md`.
3. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json`.
4. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`.
5. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C1-spec.md`.
6. Set task status (`C1-code`) to `in_progress` in `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json` (orchestration branch only).
7. Add START entry to `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`; commit docs (`docs: start C1-code`).
8. Create worktree: `git worktree add -b iu4-c1-iu-roots-code wt/iu4-c1-iu-roots-code feat/codex-wrapper-iu-subtree-inheritance`.
9. Do not edit docs/tasks/session_log from the worktree.

## End Checklist
1. Run: `cargo fmt`
2. Run: `cargo clippy --workspace --all-targets -- -D warnings`
3. Commit changes inside `wt/iu4-c1-iu-roots-code` (no docs/tasks/session_log edits).
4. Checkout `feat/codex-wrapper-iu-subtree-inheritance`; set `C1-code` to `completed`; add END entry; commit docs (`docs: finish C1-code`).
5. Remove worktree.

