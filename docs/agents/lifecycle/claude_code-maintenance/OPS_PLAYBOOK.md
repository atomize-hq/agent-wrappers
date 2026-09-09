<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# Ops playbook

This packet-owned playbook freezes operator context for `claude_code` target `2.1.236`.

- request artifact: `docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml`
- basis ref: `cli_manifests/claude_code/latest_validated.txt`
- opened from: `.github/workflows/agent-maintenance-open-pr.yml`
- branch linkage: `automation/claude_code-maintenance-2.1.236`
- canonical handoff: `docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md`
- recovery packet regeneration: `cargo run -p xtask -- refresh-agent --request docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml --write`
