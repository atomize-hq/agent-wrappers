<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# Scope brief

This automated maintenance lane is limited to the frozen shared packet for `claude_code` and the declared writable surfaces below.

## Support uplift rule

Newly discovered non-TUI surface must land in this run unless the frozen packet records one allowed deferral. Preexisting non-TUI gaps remain valid only when the packet ties them to the committed debt inventory.

## Writable surfaces

- `docs/agents/lifecycle/claude_code-maintenance/**`
- `crates/claude_code/**`
- `crates/agent_api/**`
- `cli_manifests/claude_code/artifacts.lock.json`
- `cli_manifests/claude_code/snapshots/2.1.236/**`
- `cli_manifests/claude_code/reports/2.1.236/**`
- `cli_manifests/claude_code/versions/2.1.236.json`
- `cli_manifests/claude_code/wrapper_coverage.json`
- `cli_manifests/support_matrix/current.json`
- `docs/specs/unified-agent-api/support-matrix.md`
- `crates/agent_api/src/runtime_support_data.rs`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`
