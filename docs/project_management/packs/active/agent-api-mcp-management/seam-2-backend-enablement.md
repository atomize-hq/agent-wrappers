# SEAM-2 — Backend enablement + safe default advertising

- **Name**: Backend enablement + safe default advertising (write ops) + isolated homes
- **Type**: integration (safety / permissions)
- **Goal / user value**: Ensure MCP management APIs are safe-by-default (no user-state mutation / no write capabilities
  advertised unless explicitly enabled) while still enabling automation via isolated homes.

## Scope

### In

- Define per-backend config that controls MCP management capability advertising, especially write ops:
  - `agent_api.tools.mcp.add.v1`
  - `agent_api.tools.mcp.remove.v1`
- Define/confirm isolated home overrides for built-in backends so MCP config mutations can be confined to a temp root.
- Ensure capability advertising and config defaults enforce “safe-by-default” posture.

### Out

- Any global “policy engine” for permissions beyond this specific MCP management surface.
- Changing upstream CLIs’ own config location rules (we only adapt via supported flags/env/config).

## Primary interfaces (contracts)

### Inputs

- Backend configuration (built-in Codex + Claude Code backends) that controls:
  - capability advertising for MCP operations,
  - state root / “home” directory selection for automation.

### Outputs

- Backends advertise only the MCP capabilities they implement and are enabled to expose.

## Key invariants / rules

- Built-in backends MUST NOT advertise `add/remove` by default.
- Capability advertising remains the source of truth for `UnsupportedCapability` gating.
- Isolated homes must ensure tests/automation do not mutate user state by default.

## Dependencies

- **Blocks**:
  - SEAM-5 (tests need stable defaults + config)
- **Blocked by**:
  - SEAM-1 (capability ids + API shape)

## Touch surface

- `crates/agent_api/src/backends/codex.rs` (capabilities + config + isolation knobs)
- `crates/agent_api/src/backends/claude_code.rs` (capabilities + config + isolation knobs)
- Potentially wrapper builders/config:
  - `crates/codex/src/**` (if Codex “home” override is owned by wrapper)
  - `crates/claude_code/src/**` (if Claude “home” override is owned by wrapper)

## Verification

- Unit tests pinning default capability advertising (write ops off by default).
- Harness/integration tests that run `list/get/add/remove` against an isolated home directory and confirm:
  - state mutations are confined to the isolated root,
  - no network access is required.

## Risks / unknowns

- **Isolation wiring surface**: pin which `agent_api` backend config fields map to:
  - Codex wrapper `CodexClientBuilder::codex_home` (`CODEX_HOME` injection), and
  - Claude wrapper `ClaudeCodeClientBuilder::claude_home` (`CLAUDE_HOME` + `HOME`/`XDG_*` injection),
  so SEAM-5 can run safely against temp roots.
  - **De-risk plan**: document the mapping explicitly in SEAM-2 and add tests that assert the correct env vars are injected.

## Rollout / safety

- Defaults must remain safe (write ops disabled unless enabled).
- Explicit enablement is discoverable (via backend config and/or advertised capabilities).
