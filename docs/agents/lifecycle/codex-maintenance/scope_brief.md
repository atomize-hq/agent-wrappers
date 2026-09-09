<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# Scope brief

This automated maintenance lane is limited to the frozen shared packet for `codex` and the declared writable surfaces below.

## Support uplift rule

Newly discovered non-TUI surface must land in this run unless the frozen packet records one allowed deferral. Preexisting non-TUI gaps remain valid only when the packet ties them to the committed debt inventory.

## Writable surfaces

- `docs/agents/lifecycle/codex-maintenance/**`
- `crates/codex/**`
- `crates/agent_api/**`
- `cli_manifests/codex/artifacts.lock.json`
- `cli_manifests/codex/snapshots/0.153.3/**`
- `cli_manifests/codex/reports/0.153.3/**`
- `cli_manifests/codex/versions/0.153.3.json`
- `cli_manifests/codex/wrapper_coverage.json`
- `cli_manifests/support_matrix/current.json`
- `docs/specs/unified-agent-api/support-matrix.md`
- `crates/agent_api/src/runtime_support_data.rs`
- `docs/specs/unified-agent-api/non-tui-support-debt.md`
- `docs/specs/codex-wrapper-coverage-scenarios-v1.md`
