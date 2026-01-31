# C0 Test Kickoff — IU Subtree Inheritance (ADR 0004)

Scope: tests/fixtures only per `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C0-spec.md`.

Role boundaries:
- Tests only (no production code).
- Required commands: `cargo fmt`; `cargo test -p xtask --test c5_spec_iu_subtree_inheritance -- --nocapture`; `cargo test -p xtask --test c6_spec_report_iu_validator -- --nocapture`.

## Start Checklist
1. `git checkout feat/codex-wrapper-iu-subtree-inheritance && git pull --ff-only`
2. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/plan.md`.
3. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json`.
4. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`.
5. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C0-spec.md`.
6. Set task status (`C0-test`) to `in_progress` in `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json` (orchestration branch only).
7. Add START entry to `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`; commit docs (`docs: start C0-test`).
8. Create worktree: `git worktree add -b iu4-c0-report-iu-inheritance-test wt/iu4-c0-report-iu-inheritance-test feat/codex-wrapper-iu-subtree-inheritance`.
9. Do not edit docs/tasks/session_log from the worktree.

## End Checklist
1. Run: `cargo fmt`
2. Run: `cargo test -p xtask --test c5_spec_iu_subtree_inheritance -- --nocapture`
3. Run: `cargo test -p xtask --test c6_spec_report_iu_validator -- --nocapture`
4. Commit changes inside `wt/iu4-c0-report-iu-inheritance-test` (no docs/tasks/session_log edits).
5. Checkout `feat/codex-wrapper-iu-subtree-inheritance`; set `C0-test` to `completed`; add END entry; commit docs (`docs: finish C0-test`).
6. Remove worktree: `git worktree remove wt/iu4-c0-report-iu-inheritance-test`.
