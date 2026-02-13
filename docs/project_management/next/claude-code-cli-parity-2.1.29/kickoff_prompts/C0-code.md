# Kickoff Prompt – C0-code (<TRIAD TITLE>)

## Scope
- Production code only; no tests. Implement the C0-spec.

## Start Checklist
1. `git checkout feat/claude-code-cli-parity-2.1.29 && git pull --ff-only`
2. Read: `plan.md`, `tasks.json`, `session_log.md`, `C0-spec.md`, this prompt.
3. Set `C0-code` status to `in_progress` in `tasks.json` (orchestration branch only).
4. Add START entry to `session_log.md`; commit docs (`docs: start C0-code`).
5. Create the task branch and worktree: `git worktree add -b claude-code-cli-parity-2-1-29-c0-v2-1-29-code wt/claude-code-cli-parity-2-1-29-c0-v2-1-29-code feat/claude-code-cli-parity-2.1.29`.
6. Do **not** edit docs/tasks/session_log.md from the worktree.

## Requirements
- Implement C0 per `C0-spec.md`.
- Protected paths: `.git`, `.substrate-git`, `.substrate`, sockets, device nodes (unless the spec explicitly says otherwise).
- Required commands (before handoff):
  - `cargo fmt`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Optional sanity checks allowed, but no required tests.

## End Checklist
1. Run the required commands above and capture their outputs.
2. Inside `wt/claude-code-cli-parity-2-1-29-c0-v2-1-29-code`, commit C0-code changes (no docs/tasks/session_log.md edits).
3. From outside the worktree, ensure the task branch contains the worktree commit (fast-forward if needed); do **not** merge into `feat/claude-code-cli-parity-2.1.29`.
4. Checkout `feat/claude-code-cli-parity-2.1.29`; update `tasks.json` to `completed`; add an END entry to `session_log.md` with commands/results/blockers; create downstream prompts if missing; commit docs (`docs: finish C0-code`).
5. Remove worktree `wt/claude-code-cli-parity-2-1-29-c0-v2-1-29-code`.


## Parity Work Queue (from coverage.any.json)
Report: `cli_manifests/claude_code/reports/2.1.29/coverage.any.json`

### Missing commands
- `install`
- `plugin disable`
- `plugin enable`
- `plugin install`
- `plugin list`
- `plugin marketplace add`
- `plugin marketplace list`
- `plugin marketplace remove`
- `plugin marketplace update`
- `plugin uninstall`
- `plugin update`
- `plugin validate`

### Missing flags
- `install --force`
- `mcp add-from-claude-desktop --scope`
- `plugin disable --all`
- `plugin disable --scope`
- `plugin enable --scope`
- `plugin install --scope`
- `plugin list --available`
- `plugin list --json`
- `plugin marketplace list --json`
- `plugin uninstall --scope`
- `plugin update --scope`

### Missing args
- `mcp add commandOrUrl`
- `mcp add name`
- `mcp add-json json`
- `mcp add-json name`
- `mcp get name`
- `mcp remove name`
- `plugin enable plugin`
- `plugin marketplace add source`
- `plugin update plugin`
- `plugin validate path`
