# Kickoff Prompt – C3-integ (Ops playbook + promotion rules)

## Scope
Merge `C3-code` + `C3-test`, reconcile to `docs/project_management/next/codex-cli-parity/C3-spec.md`, and ensure the triad is green. Integration owns aligning code/tests to the spec.

Expected deliverables (exact paths):
- Ops playbook: `cli_manifests/codex/OPS_PLAYBOOK.md`
- Link from: `cli_manifests/codex/README.md` (section `## Ops Playbook`)

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C3-spec.md`, this prompt.
3. Set `C3-integ` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C3-integ`).
5. Create the integration branch and worktree: `git worktree add -b ccp-c3-ops-integ wt/ccp-c3-ops-integ feat/codex-cli-parity`.
6. Do **not** edit docs/tasks/session_log from the worktree.

## Requirements
- Merge branches `ccp-c3-ops-code` + `ccp-c3-ops-test` and reconcile behavior to `docs/project_management/next/codex-cli-parity/C3-spec.md`.
- Run required commands (capture outputs in END log):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p codex`
  - Integration gate: `make preflight`

## End Checklist
1. Merge upstream C3 code/test branches into the integration worktree and reconcile behavior to the spec.
2. Run required commands above and capture outputs.
3. Commit integration changes on branch `ccp-c3-ops-integ`.
4. Fast-forward merge `ccp-c3-ops-integ` into `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C3-integ`).
5. Remove worktree `wt/ccp-c3-ops-integ`.
