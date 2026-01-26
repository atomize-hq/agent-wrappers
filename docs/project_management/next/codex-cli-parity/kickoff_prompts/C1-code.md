# Kickoff Prompt – C1-code (Version policy + CI workflows)

## Scope
Implement `docs/project_management/next/codex-cli-parity/C1-spec.md` (non-test changes only): CI/workflow automation for real-binary validation + artifacts lockfile semantics. No tests.

Expected deliverables (exact paths):
- `.github/workflows/ci.yml`
- `.github/workflows/codex-cli-release-watch.yml`
- `.github/workflows/codex-cli-update-snapshot.yml`
- `cli_manifests/codex/artifacts.lock.json`

Role boundaries:
- Code agent: non-test changes only; no tests; no live/credentialed flows.
- Must not introduce crate-runtime binary auto-download/auto-update behavior (downloads happen only in CI/workflows per spec).

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/adr/0001-codex-cli-parity-maintenance.md`, `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C1-spec.md`, this prompt.
3. Set `C1-code` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C1-code`).
5. Create the task branch and worktree: `git worktree add -b ccp-c1-validation-code wt/ccp-c1-validation-code feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Implement C1 per `docs/project_management/next/codex-cli-parity/C1-spec.md`.
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccp-c1-validation-code`, commit C1-code changes (no planning-pack docs edits).
3. From outside the worktree, ensure branch `ccp-c1-validation-code` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity`.
4. Checkout `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C1-code`).
5. Remove worktree `wt/ccp-c1-validation-code`.
