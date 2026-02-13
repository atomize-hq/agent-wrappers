# C0-spec – Parity update for `claude_code` `2.1.29`

## Scope
- Use the generated coverage report as the work queue.
- Report: `cli_manifests/claude_code/reports/2.1.29/coverage.any.json`
- Implement wrapper support or explicitly waive with `intentionally_unsupported` notes.
- Regenerate artifacts and pass `codex-validate` for the parity root.

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

## Acceptance Criteria
- Wrapper changes address C0 scope.
- Artifacts regenerated deterministically.
- `cargo run -p xtask -- codex-validate --root <root>` passes.

## Out of Scope
- Promotion (pointer/current.json updates) unless explicitly requested.
