<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# Ops playbook

This packet-owned playbook freezes operator context for `codex` target `0.153.3`.

- request artifact: `docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml`
- basis ref: `cli_manifests/codex/latest_validated.txt`
- opened from: `.github/workflows/agent-maintenance-open-pr.yml`
- branch linkage: `automation/codex-maintenance-0.153.3`
- canonical handoff: `docs/agents/lifecycle/codex-maintenance/HANDOFF.md`
- recovery packet regeneration: `cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/codex-maintenance/governance/maintenance-request.toml --write`
