# C1 Test Kickoff — Adopt IU Roots

Scope: tests/fixtures only per `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C1-spec.md`.

Role boundaries:
- Tests only (no production code).
- Required commands: `cargo fmt`; `cargo test -p xtask --test c7_spec_iu_roots_adoption -- --nocapture`.

## Start Checklist
1. `git checkout feat/codex-wrapper-iu-subtree-inheritance && git pull --ff-only`
2. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/plan.md`.
3. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json`.
4. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`.
5. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C1-spec.md`.
6. Confirm C1 code branch (`iu4-c1-iu-roots-code`) exists before starting (tests rely on the IU roots being present in the build).
7. Set task status (`C1-test`) to `in_progress` in `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json` (orchestration branch only).
8. Add START entry to `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`; commit docs (`docs: start C1-test`).
9. Create worktree: `git worktree add -b iu4-c1-iu-roots-test wt/iu4-c1-iu-roots-test feat/codex-wrapper-iu-subtree-inheritance`.
10. Do not edit docs/tasks/session_log from the worktree.

## End Checklist
1. Run: `cargo fmt`
2. Run: `cargo test -p xtask --test c7_spec_iu_roots_adoption -- --nocapture`
3. Commit changes inside `wt/iu4-c1-iu-roots-test` (no docs/tasks/session_log edits).
4. Checkout `feat/codex-wrapper-iu-subtree-inheritance`; set `C1-test` to `completed`; add END entry; commit docs (`docs: finish C1-test`).
5. Remove worktree.
