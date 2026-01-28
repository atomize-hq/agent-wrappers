# Kickoff Prompt – C0-code (Deterministic validators)

## Scope
- Production code only; no tests. Implement `xtask codex-validate` per `docs/project_management/next/codex-cli-parity-coverage-mapping/C0-spec.md`.

## Start Checklist
1. `git checkout feat/codex-cli-parity-coverage-mapping && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity-coverage-mapping/plan.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json`, `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`, `docs/project_management/next/codex-cli-parity-coverage-mapping/C0-spec.md`, this prompt.
3. Set `C0-code` status to `in_progress` in `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md`; commit docs (`docs: start C0-code`).
5. Create the task branch and worktree: `git worktree add -b ccm-c0-validate-code wt/ccm-c0-validate-code feat/codex-cli-parity-coverage-mapping`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` or `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` from the worktree.

## Requirements
- Implement C0 per `docs/project_management/next/codex-cli-parity-coverage-mapping/C0-spec.md`.
- Protected paths: `.git`, `target/`, `.substrate-git`, `.substrate`, sockets, device nodes (unless the spec explicitly says otherwise).
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Optional sanity checks allowed, but no required tests.

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccm-c0-validate-code`, commit C0-code changes (no planning-pack docs edits).
3. From outside the worktree, ensure branch `ccm-c0-validate-code` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity-coverage-mapping`.
4. Checkout `feat/codex-cli-parity-coverage-mapping`; update `docs/project_management/next/codex-cli-parity-coverage-mapping/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity-coverage-mapping/session_log.md` with commands/results/blockers; commit docs (`docs: finish C0-code`).
5. Remove worktree `wt/ccm-c0-validate-code`.
