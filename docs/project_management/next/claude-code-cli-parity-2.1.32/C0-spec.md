# C0-spec – Parity update for `claude_code` `2.1.32`

## Scope
- Use the generated coverage report as the work queue.
- Report: `cli_manifests/claude_code/reports/2.1.32/coverage.any.json`
- Implement wrapper support or explicitly waive with `intentionally_unsupported` notes.
- Regenerate artifacts and pass `codex-validate` for the parity root.

### Missing commands
- (none)

### Missing flags
- `mcp add --callback-port`
- `mcp add --client-id`
- `mcp add --client-secret`
- `mcp add-json --client-secret`

### Missing args
- (none)

## Acceptance Criteria
- Wrapper changes address C0 scope.
- Artifacts regenerated deterministically.
- `cargo run -p xtask -- codex-validate --root <root>` passes.

## Out of Scope
- Promotion (pointer/current.json updates) unless explicitly requested.
