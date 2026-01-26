# Kickoff Prompt – C0-code (Snapshot schema + generator)

## Scope
Implement `docs/project_management/next/codex-cli-parity/C0-spec.md` (non-test changes only): snapshot schema v1 + deterministic snapshot generator; no tests.

Expected deliverables (exact paths):
- New crate: `crates/xtask/` (package `xtask`, binary `xtask`)
- Snapshot schema docs: `cli_manifests/codex/README.md` (update existing)
- Supplement baseline: `cli_manifests/codex/supplement/commands.json`
- Generator output target (do not commit generated output unless the spec requires it): `cli_manifests/codex/current.json` and optional `cli_manifests/codex/raw_help/<version>/**`

## Start Checklist
1. `git checkout feat/codex-cli-parity && git pull --ff-only`
2. Read: `docs/project_management/next/codex-cli-parity/plan.md`, `docs/project_management/next/codex-cli-parity/tasks.json`, `docs/project_management/next/codex-cli-parity/session_log.md`, `docs/project_management/next/codex-cli-parity/C0-spec.md`, this prompt.
3. Set `C0-code` status to `in_progress` in `docs/project_management/next/codex-cli-parity/tasks.json` (orchestration branch only).
4. Add START entry to `docs/project_management/next/codex-cli-parity/session_log.md`; commit docs (`docs: start C0-code`).
5. Create the task branch and worktree: `git worktree add -b ccp-c0-snapshot-code wt/ccp-c0-snapshot-code feat/codex-cli-parity`.
6. Do **not** edit `docs/project_management/next/codex-cli-parity/tasks.json` or `docs/project_management/next/codex-cli-parity/session_log.md` from the worktree.

## Requirements
- Implement C0 per `docs/project_management/next/codex-cli-parity/C0-spec.md`.
- Protected paths: `.git`, `.substrate-git`, `.substrate`, sockets, device nodes (unless the spec explicitly says otherwise).
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Optional sanity checks allowed, but no required tests.

Canonical generator command (must be documented in `cli_manifests/codex/README.md`):
- `cargo run -p xtask -- codex-snapshot --codex-binary <PATH_TO_CODEX> --out-dir cli_manifests/codex --capture-raw-help --supplement cli_manifests/codex/supplement/commands.json`

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/ccp-c0-snapshot-code`, commit C0-code changes (no planning-pack docs edits).
3. From outside the worktree, ensure branch `ccp-c0-snapshot-code` contains the worktree commit (fast-forward if needed); do **not** merge into `feat/codex-cli-parity`.
4. Checkout `feat/codex-cli-parity`; update `docs/project_management/next/codex-cli-parity/tasks.json` to `completed`; add an END entry to `docs/project_management/next/codex-cli-parity/session_log.md` with commands/results/blockers; commit docs (`docs: finish C0-code`).
5. Remove worktree `wt/ccp-c0-snapshot-code`.
