# Scope brief — Universal MCP management commands (add/get/list/remove)

## Goal

Introduce a **minimal, universal, non-run** MCP management API in `agent_api` so orchestrators can manage MCP
server configurations across built-in backends without depending on backend-specific wrapper crates.

Universalized v1 operations:

- `list`
- `get`
- `add`
- `remove`

## Why now

Orchestrators need a backend-neutral way to ensure MCP servers are present/removed **before** a run.
Treating this as `AgentWrapperRunRequest.extensions` is a category error: MCP management is a separate CLI command tree.

## Primary users + JTBD

- **Host integrators / orchestrators**: “Ensure required MCP tool servers are configured (or removed) for a given backend
  as part of unattended automation.”

## In-scope

- Implement the public `agent_api::mcp` module and pinned type shapes from:
  - `docs/specs/universal-agent-api/mcp-management-spec.md`
- Capability-gated gateway entrypoints:
  - `AgentWrapperGateway::{mcp_list,mcp_get,mcp_add,mcp_remove}`
- Backend hooks:
  - default `AgentWrapperBackend::{mcp_list,mcp_get,mcp_add,mcp_remove}` methods that fail-closed with
    `UnsupportedCapability` unless implemented.
- Built-in backend mappings (Codex + Claude Code) for `add/get/list/remove`.
- Safe-by-default posture:
  - **write ops require explicit enablement** (do not advertise `add/remove` by default),
  - support isolated homes so automation can avoid mutating user state,
  - bounded stdout/stderr capture + deterministic truncation markers,
  - MCP management outputs MUST NOT be emitted as run events.

## Out-of-scope

- Standardizing a universal structured MCP config schema (v1 returns bounded stdout/stderr).
- Guaranteeing output format parity across backends (e.g., `--json` vs text).
- Universalizing backend-specific MCP extras (Codex `login/logout`; Claude `add-json`, `serve`, etc.).
- Modeling MCP management as run extensions under `AgentWrapperRunRequest.extensions`.

## Capability inventory (implied)

- Capability ids (v1):
  - `agent_api.tools.mcp.list.v1`
  - `agent_api.tools.mcp.get.v1`
  - `agent_api.tools.mcp.add.v1`
  - `agent_api.tools.mcp.remove.v1`
- Request validation:
  - names trimmed + non-empty,
  - `Stdio.command` non-empty,
  - output budgets applied uniformly.
- Backend config for safe default advertising and isolated homes.

## Required invariants (must not regress)

- **Fail-closed capability gating**: invoking an unadvertised operation returns
  `AgentWrapperError::UnsupportedCapability { agent_kind, capability }`.
- **Non-run boundary**: MCP management outputs MUST NOT be emitted as `AgentWrapperEvent`s.
- **Bounded outputs**: stdout/stderr captured with fixed budgets (65,536 bytes each) and deterministic truncation marker:
  `…(truncated)` (UTF-8 preserved).
- **Parent env safety**: per-request env overrides apply only to spawned backend processes; parent process env is not mutated.
- **Safe-by-default advertising**: built-in backends MUST NOT advertise `add/remove` unless explicitly enabled.

## Success criteria

- A caller can invoke `mcp_list/get/add/remove` through `AgentWrapperGateway` for a chosen backend kind.
- By default, built-in backends advertise read operations (`list/get`) when supported on this target and do **not** advertise
  write operations (`add/remove`) unless explicitly enabled (`allow_mcp_write=true`; pinned in SEAM-2).
- All operations enforce request validation and output bounds.
- Automation can run against an isolated home to avoid mutating user state.

## Constraints

- Public API uses std + serde-friendly types only (no `codex::*` / `claude_code::*` in public types).
- The API is typed/bounded (no generic argv pass-through; no “extra args” escape hatch).
- No network access is required for tests.

## External systems / dependencies

- Upstream CLIs:
  - `codex mcp {add,get,list,remove}` (plus extras)
  - `claude mcp {add,get,list,remove}` (plus extras)
- CLI inventory snapshots:
  - `cli_manifests/codex/current.json`
  - `cli_manifests/claude_code/current.json`

## Known unknowns / risks

- **Claude URL auth mapping (resolved)**: pinned behavior is to reject `Url.bearer_token_env_var` as `InvalidRequest` for
  Claude (no deterministic/safe mapping to `claude mcp add --header` in v1; see SEAM-4).
- **Claude subcommand availability variance**: the CLI manifest snapshot appears to omit some MCP subcommands on non-Windows
  targets; confirm real-world CLI parity and avoid relying on snapshot artifacts.
- **Isolated home wiring (resolved)**: pinned backend config fields and wrapper mapping live in SEAM-2 (`codex_home` /
  `claude_home`; no parent env mutation).

## Assumptions (explicit)

- Built-in backends gate write operations behind an explicit backend config flag `allow_mcp_write: bool` (default `false`)
  and advertise capabilities accordingly (pinned in SEAM-2 and `docs/specs/universal-agent-api/contract.md`).
- For v1, `agent_api` returns bounded stdout/stderr as-is and does not attempt to normalize or redact backend-specific formats.
