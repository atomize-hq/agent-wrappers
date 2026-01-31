# C0 Integration Kickoff — IU Subtree Inheritance (ADR 0004)

Scope: merge C0 code+test, reconcile to `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C0-spec.md`, and gate.

Role boundaries:
- Integration only (merge + reconcile to spec; edit prod/tests only when required to satisfy `C0-spec.md`).
- Required commands (no substitutions): `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p xtask --test c5_spec_iu_subtree_inheritance -- --nocapture`; `cargo test -p xtask --test c6_spec_report_iu_validator -- --nocapture`; `make preflight`.

## Start Checklist
1. `git checkout feat/codex-wrapper-iu-subtree-inheritance && git pull --ff-only`
2. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/plan.md`.
3. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json`.
4. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`.
5. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C0-spec.md`.
6. Set task status (`C0-integ`) to `in_progress` in `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json` (orchestration branch only).
7. Add START entry to `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`; commit docs (`docs: start C0-integ`).
8. Create worktree: `git worktree add -b iu4-c0-report-iu-inheritance-integ wt/iu4-c0-report-iu-inheritance-integ feat/codex-wrapper-iu-subtree-inheritance`.
9. Do not edit docs/tasks/session_log from the worktree.

## End Checklist
1. Merge branches: `iu4-c0-report-iu-inheritance-code` and `iu4-c0-report-iu-inheritance-test`.
2. Set deterministic timestamp for generated artifacts: `export SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)"`
3. Run: `cargo fmt`
4. Run: `cargo clippy --workspace --all-targets -- -D warnings`
5. Run: `cargo test -p xtask --test c5_spec_iu_subtree_inheritance -- --nocapture`
6. Run: `cargo test -p xtask --test c6_spec_report_iu_validator -- --nocapture`
7. Run: `make preflight`
6. Commit inside `wt/iu4-c0-report-iu-inheritance-integ`.
7. Fast-forward merge `iu4-c0-report-iu-inheritance-integ` into `feat/codex-wrapper-iu-subtree-inheritance`.
8. Update `tasks.json` (`C0-integ` → `completed`) and add END entry; commit docs (`docs: finish C0-integ`).
9. Remove worktree.
