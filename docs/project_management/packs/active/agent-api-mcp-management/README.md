# Universal MCP management commands (add/get/list/remove) — seam extraction (ADR-0018)

Source ADR: `docs/adr/0018-universal-mcp-management-commands.md`  
Canonical draft spec (normative once approved): `docs/specs/universal-agent-api/mcp-management-spec.md`  
Backlog: `uaa-0006` (`bucket=agent_api.tools`, `type=api_surface`)

This directory contains **seam** artifacts extracted to make the work owner-assignable and parallelizable without hiding coupling.
These files are planning aids; they are not normative contracts (authoritative contracts remain in `docs/specs/**`).

Note: The MCP management spec is currently **Draft**. This pack treats it as the canonical implementation target; changes
should be made in the spec (and any other canonical contracts) first, then reflected here.

- Start here: `scope_brief.md`
- Seam overview: `seam_map.md`
- Threading (contracts + dependencies + workstreams): `threading.md`

## Canonical contracts (source of truth)

- MCP management contract + pinned types/budgets: `docs/specs/universal-agent-api/mcp-management-spec.md`
- Capability gating + errors: `docs/specs/universal-agent-api/contract.md`
- Capability naming + schema posture: `docs/specs/universal-agent-api/capabilities-schema-spec.md`
- Output bounds precedent (text budgets): `docs/specs/universal-agent-api/event-envelope-schema-spec.md`
- Posture / promotion criteria: `docs/adr/0001-codex-cli-parity-maintenance.md`
