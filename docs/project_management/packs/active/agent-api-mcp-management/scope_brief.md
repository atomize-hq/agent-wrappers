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
  - transport validation occurs before any spawn (InvalidRequest; safe/redacted messages):
    - `Stdio`: `command` non-empty; every item in `command` and `args` trimmed + non-empty; `argv = command + args`.
    - `Url`: `url` trimmed + non-empty; parse absolute `http`/`https` URL; `bearer_token_env_var` (if present) trimmed +
      non-empty and matches `^[A-Za-z_][A-Za-z0-9_]*$`.
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
- **Manifest snapshot drift**: pinned CLI manifest snapshots are normative for v1 advertising; if runtime behavior conflicts
  with the snapshot, the operation MUST fail as `AgentWrapperError::Backend` and the backend MUST NOT silently mutate its
  advertised capabilities (remediation is a repo update to the pinned manifests + mapping).

## Success criteria

- A caller can invoke `mcp_list/get/add/remove` through `AgentWrapperGateway` for a chosen backend kind.
- By default, built-in backends advertise read operations (`list/get`) when supported on this target and do **not** advertise
  write operations (`add/remove`) unless explicitly enabled via the public built-in config fields
  (`CodexBackendConfig.allow_mcp_write=true` / `ClaudeCodeBackendConfig.allow_mcp_write=true`;
  pinned in SEAM-2 and `docs/specs/universal-agent-api/contract.md`).
- All operations enforce request validation and output bounds.
- Automation can run against an isolated home to avoid mutating user state.
- The generated default capability matrix may omit `agent_api.tools.mcp.add.v1` /
  `agent_api.tools.mcp.remove.v1` because it is built from default built-in configs; runtime truth
  remains the selected backend instance's `capabilities().ids`.

## Constraints

- Public API uses std + serde-friendly types only (no `codex::*` / `claude_code::*` in public types). Canonical definition:
  `docs/specs/universal-agent-api/contract.md#serde-friendly-types`.
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
- **Claude subcommand availability variance (pinned for v1)**: the pinned CLI manifest snapshot shows `mcp get/add/remove`
  only on `win32-x64`. Treat the manifest snapshots as the authoritative source of truth for v1 capability advertising and
  target availability gating. If observed upstream CLI behavior differs at runtime, treat it as drift/bug:
  - the backend MUST NOT silently change advertised capabilities at runtime, and
  - the operation MUST fail as `AgentWrapperError::Backend` (not `UnsupportedCapability`).
  Remediation is to update the pinned manifests + mapping in a follow-up repo change.
- **Isolated home wiring (resolved)**: pinned backend config fields and wrapper mapping live in SEAM-2 (`codex_home` /
  `claude_home`; no parent env mutation).

## Assumptions (explicit)

- Built-in backends gate write operations behind an explicit backend config flag `allow_mcp_write: bool` (default `false`)
  and advertise capabilities accordingly. The canonical v1 fields are
  `agent_api::backends::codex::CodexBackendConfig.allow_mcp_write` and
  `agent_api::backends::claude_code::ClaudeCodeBackendConfig.allow_mcp_write` (pinned in SEAM-2
  and `docs/specs/universal-agent-api/contract.md`).
- For v1, `agent_api` returns bounded stdout/stderr as-is and does not attempt to normalize or redact backend-specific formats.
