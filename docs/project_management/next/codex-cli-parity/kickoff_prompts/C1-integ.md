# Kickoff Prompt – C1-integ (Version policy + CI workflows)

## Scope
Merge `C1-code` + `C1-test`, reconcile to `docs/project_management/next/codex-cli-parity/C1-spec.md`, and ensure the triad is green. Integration owns aligning code/tests to the spec.

Expected deliverables (exact paths):
- `.github/workflows/ci.yml`
- `.github/workflows/codex-cli-release-watch.yml`
- `.github/workflows/codex-cli-update-snapshot.yml`
- `cli_manifests/codex/artifacts.lock.json`

Role boundaries:
- Integration agent: merges code+tests, reconciles to spec, and runs required gates including `make preflight`.
- Must ensure validations reflect the ADR “validated” definition (Linux, real-binary `cli_e2e` with isolated `CODEX_HOME`).

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C1-spec.md`, this prompt.
3. Set `C1-integ` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C1-integ`).
5. Create the integration branch and worktree: `git worktree add -b ccp-c1-validation-integ wt/ccp-c1-validation-integ feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Merge branches `ccp-c1-validation-code` + `ccp-c1-validation-test` and reconcile behavior to `docs/project_management/next/codex-cli-parity/C1-spec.md`.
- Ensure `./codex-x86_64-unknown-linux-musl` exists before running `cli_e2e` (download/extract the upstream `codex-x86_64-unknown-linux-musl.tar.gz` release asset, or run the Update Snapshot workflow which performs the same download in CI).
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p codex`
  - `cargo test -p codex --examples`
  - Real-binary E2E (explicit isolated home; set `CODEX_E2E_BINARY` to the binary under validation):
    - `export CODEX_E2E_HOME=$(mktemp -d) && export CODEX_HOME=$CODEX_E2E_HOME && CODEX_E2E_BINARY=./codex-x86_64-unknown-linux-musl cargo test -p codex --test cli_e2e -- --nocapture`
  - Integration gate: `make preflight`

## End Checklist
1. Merge upstream C1 code/test branches into the integration worktree and reconcile behavior to the spec.
2. Run required commands above and capture outputs.
3. Commit integration changes on branch `ccp-c1-validation-integ`.
4. Fast-forward merge `ccp-c1-validation-integ` into `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C1-integ`).
5. Remove worktree `wt/ccp-c1-validation-integ`.
