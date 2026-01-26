# Kickoff Prompt – C3-code (Ops playbook + promotion rules)

## Scope
Implement `docs/project_management/next/codex-cli-parity/C3-spec.md` (non-test changes only): maintainer runbook + explicit policy documentation for intentionally unwrapped surfaces and promotion criteria. No tests.

Expected deliverables (exact paths):
- Ops playbook: `cli_manifests/codex/OPS_PLAYBOOK.md`
- Link from: `cli_manifests/codex/README.md` (section `## Ops Playbook`)

Role boundaries:
- Code agent: production (non-test) changes only; no tests.

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C3-spec.md`, this prompt.
3. Set `C3-code` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C3-code`).
5. Create the task branch and worktree: `git worktree add -b ccp-c3-ops-code wt/ccp-c3-ops-code feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Implement C3 per `docs/project_management/next/codex-cli-parity/C3-spec.md`.
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccp-c3-ops-code`, commit C3-code changes (no planning-pack docs edits).
3. From outside the worktree, ensure branch `ccp-c3-ops-code` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity`.
4. Checkout `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C3-code`).
5. Remove worktree `wt/ccp-c3-ops-code`.
