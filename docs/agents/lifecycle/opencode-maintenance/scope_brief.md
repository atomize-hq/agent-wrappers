<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# Scope brief

This automated maintenance lane is limited to the frozen shared packet for `opencode` and the declared writable surfaces below.

## Support uplift rule

Newly discovered non-TUI surface must land in this run unless the frozen packet records one allowed deferral. Preexisting non-TUI gaps remain valid only when the packet ties them to the committed debt inventory.

## Writable surfaces

- `docs/agents/lifecycle/opencode-maintenance/**`
- `crates/opencode/**`
- `crates/agent_api/**`
- `cli_manifests/opencode/artifacts.lock.json`
- `cli_manifests/opencode/snapshots/1.18.28/**`
- `cli_manifests/opencode/reports/1.18.28/**`
- `cli_manifests/opencode/versions/1.18.28.json`
- `cli_manifests/opencode/wrapper_coverage.json`
- `cli_manifests/support_matrix/current.json`
- `docs/specs/unified-agent-api/support-matrix.md`
- `crates/agent_api/src/runtime_support_data.rs`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`
