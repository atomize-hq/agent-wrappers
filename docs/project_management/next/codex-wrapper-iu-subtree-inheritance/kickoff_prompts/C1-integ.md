# C1 Integration Kickoff — Adopt IU Roots

Scope: merge C1 code+test, reconcile to `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C1-spec.md`, and gate.

Role boundaries:
- Integration only (merge + reconcile to spec; edit prod/tests only when required to satisfy `C1-spec.md`).
- Required commands (no substitutions): `cargo fmt`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p xtask --test c7_spec_iu_roots_adoption -- --nocapture`; `make preflight`; `cargo run -p xtask -- codex-wrapper-coverage --out cli_manifests/codex/wrapper_coverage.json`; `cargo run -p xtask -- codex-validate --root cli_manifests/codex`; plus the exact `codex-report` regeneration loop in the End Checklist.

## Start Checklist
1. `git checkout feat/codex-wrapper-iu-subtree-inheritance && git pull --ff-only`
2. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/plan.md`.
3. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json`.
4. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`.
5. Read `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/C1-spec.md`.
6. Set task status (`C1-integ`) to `in_progress` in `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/tasks.json` (orchestration branch only).
7. Add START entry to `docs/project_management/next/codex-wrapper-iu-subtree-inheritance/session_log.md`; commit docs (`docs: start C1-integ`).
8. Create worktree: `git worktree add -b iu4-c1-iu-roots-integ wt/iu4-c1-iu-roots-integ feat/codex-wrapper-iu-subtree-inheritance`.
9. Do not edit docs/tasks/session_log from the worktree.

## End Checklist
1. Merge branches: `iu4-c1-iu-roots-code` and `iu4-c1-iu-roots-test`.
2. Set deterministic timestamp for generated artifacts: `export SOURCE_DATE_EPOCH="$(git log -1 --format=%ct)"`
3. Run: `cargo fmt`
4. Run: `cargo clippy --workspace --all-targets -- -D warnings`
5. Run: `cargo test -p xtask --test c7_spec_iu_roots_adoption -- --nocapture`
6. Regenerate and validate artifacts (copy/paste; run from repo root):
   - `cargo run -p xtask -- codex-wrapper-coverage --out cli_manifests/codex/wrapper_coverage.json`
   - Regenerate reports for all committed report versions:
     - `for dir in cli_manifests/codex/reports/*; do V="$(basename "$dir")"; cargo run -p xtask -- codex-report --version "$V" --root cli_manifests/codex; done`
   - `cargo run -p xtask -- codex-validate --root cli_manifests/codex`
7. Run: `make preflight`
8. Commit inside `wt/iu4-c1-iu-roots-integ`.
9. Fast-forward merge `iu4-c1-iu-roots-integ` into `feat/codex-wrapper-iu-subtree-inheritance`.
10. Update `tasks.json` (`C1-integ` → `completed`) and add END entry; commit docs (`docs: finish C1-integ`).
11. Remove worktree.
