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
- Manifest snapshots are normative for v1 advertising (pinned by `cli_manifests/**/current.json`). If the observed upstream
  CLI behavior at runtime conflicts with the pinned snapshot, the operation MUST fail as `AgentWrapperError::Backend` and
  the backend MUST NOT silently mutate its advertised capabilities (remediation is a follow-up repo update to the pinned
  manifests + mapping).

## Pinned defaults (capability advertising)

This table is **derived guidance** for implementation and tests.

Canonical source of truth (normative once approved):
- `docs/specs/universal-agent-api/contract.md` → “MCP management write enablement (v1, normative)”
- `docs/specs/universal-agent-api/mcp-management-spec.md` → “Built-in backend behavior” → “Default capability advertising posture”
- `docs/specs/universal-agent-api/capabilities-schema-spec.md` → `agent_api.tools.mcp.{add,remove}.v1`

Legend:
- ✅ = advertised by default (when the upstream CLI subcommand is available on this target)
- ❌ = not advertised by default

| Backend | Target availability (pinned by CLI manifest) | `list` | `get` | `add` | `remove` |
| --- | --- | --- | --- | --- | --- |
| Codex (`codex`) | `cli_manifests/codex/current.json` | ✅ | ✅ | ❌ (requires `allow_mcp_write=true`) | ❌ (requires `allow_mcp_write=true`) |
| Claude Code (`claude_code`) | `cli_manifests/claude_code/current.json` | ✅ | ✅ on `win32-x64` only | ❌ (requires `win32-x64` **and** `allow_mcp_write=true`) | ❌ (requires `win32-x64` **and** `allow_mcp_write=true`) |

Notes:
- Read operations (`list/get`) have no additional enablement knob in v1. If the upstream CLI exposes the subcommand on
  this target, the backend advertises the capability by default.
- Write operations (`add/remove`) are *always* gated behind explicit backend config opt-in (see next section), even when
  the upstream CLI supports the subcommand.
- The generated capability matrix is built from default built-in backend configs. Because
  `allow_mcp_write` defaults to `false`, `docs/specs/universal-agent-api/capability-matrix.md` may
  omit `agent_api.tools.mcp.add.v1` / `agent_api.tools.mcp.remove.v1`; runtime truth is the
  selected backend instance's `capabilities().ids`.
- `allow_mcp_write` governs only non-run MCP management config mutation. It does not change what an
  MCP server can do during a normal run.

## Pinned backend config knobs (host-controlled)

Canonical host-facing config surface: `docs/specs/universal-agent-api/contract.md`.

- Write enablement (v1):
  - `agent_api::backends::codex::CodexBackendConfig.allow_mcp_write: bool` (default: `false`)
  - `agent_api::backends::claude_code::ClaudeCodeBackendConfig.allow_mcp_write: bool` (default: `false`)
  - This is the approved built-in write-enable mechanism in the canonical v1 contract.
  - When `false`, built-in backends MUST NOT advertise:
    - `agent_api.tools.mcp.add.v1`
    - `agent_api.tools.mcp.remove.v1`
- Isolated home wiring (v1):
  - Codex: `CodexBackendConfig.codex_home: Option<PathBuf>` → wrapper `CodexClientBuilder::codex_home(...)` (injects
    `CODEX_HOME` for subprocesses; parent env is never mutated).
  - Claude: `ClaudeCodeBackendConfig.claude_home: Option<PathBuf>` → wrapper `ClaudeClientBuilder::claude_home(...)`
    (injects `CLAUDE_HOME`, `HOME`, `XDG_{CONFIG,DATA,CACHE}_HOME`, and Windows equivalents for subprocesses; parent env
    is never mutated).
  - Request-level env overrides (`context.env`) are still applied per the universal config precedence rules (request keys
    win). Tests should assume that explicitly overriding `HOME`/`XDG_*` can intentionally defeat isolation.

## Advertising precedence (pinned)

For each backend instance and operation:

1) **Upstream availability**: determine whether the upstream CLI exposes the required subcommand on this target (per the
   pinned CLI manifest snapshots).
2) **Config enablement**:
   - `list/get`: enabled iff available.
   - `add/remove`: enabled iff available **and** `allow_mcp_write == true`.
3) **Advertise**: include the capability id in `capabilities().ids` iff the operation is enabled.

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
- Unit tests pinning the v1 advertising table above (including Claude target-availability gating).
- Harness/integration tests that run `list/get/add/remove` against an isolated home directory and confirm:
  - state mutations are confined to the isolated root,
  - no network access is required.
- Deterministic drift test: simulate a pinned-manifest mismatch (fake binary rejects a pinned subcommand/flag) and assert
  the operation fails as `Err(Backend)` (not `UnsupportedCapability`) without mutating advertised capabilities.

## Risks / unknowns

- None (pinned: default advertising table + `allow_mcp_write` + `codex_home`/`claude_home` wiring).

## Rollout / safety

- Defaults must remain safe (write ops disabled unless enabled).
- Explicit enablement is discoverable (via backend config and/or advertised capabilities).
