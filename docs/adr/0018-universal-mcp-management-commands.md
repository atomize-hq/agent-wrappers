# ADR-0018 — Universal MCP management commands (add/get/list/remove)
#
# Note: Run `make adr-fix ADR=docs/adr/0018-universal-mcp-management-commands.md`
# after editing to update the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft (implementation plan; normative semantics are pinned in the MCP management spec draft)
- Date (UTC): 2026-03-02
- Owner(s): spensermcconnell

## Scope

Define a **minimal, universal** MCP management surface across built-in backends:

- `add`
- `get`
- `list`
- `remove`

This work item corresponds to backlog id `uaa-0006` (`bucket=agent_api.tools`, `type=api_surface`).

This ADR explicitly covers **non-run** operations: these are CLI subcommands, not `run` extensions.

## Related Docs

- Backlog:
  - `docs/backlog.json` (`uaa-0006`)
- Universal Agent API baselines:
  - `docs/adr/0009-universal-agent-api.md`
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/extensions-spec.md` (non-goal: do not model this via run extensions)
- CLI inventory inputs:
  - `cli_manifests/codex/current.json` (`codex mcp ...`)
  - `cli_manifests/claude_code/current.json` (`claude mcp ...`)
- Wrapper library surfaces (implementation targets):
  - `crates/codex/src/commands/mcp.rs`
  - `crates/codex/src/cli/mcp.rs`
  - `crates/claude_code/src/commands/mcp.rs`
  - `crates/claude_code/src/client/mod.rs` (`mcp_*` methods)
- Posture/precedent:
  - `docs/adr/0001-codex-cli-parity-maintenance.md` ("intentionally unwrapped" MCP management commands, with promotion criteria)

## Executive Summary (Operator)

ADR_BODY_SHA256: ed679f96b44845b497542edce8863b7f050691adbae79d05a5f41bc5478db457

### Decision (draft)

- Add a **non-run**, capability-gated MCP management API to `agent_api` so callers can manage MCP
  servers across backends without depending on backend-specific wrapper crates.
- Universalize only the minimal common command set: `add/get/list/remove`.
  - Codex-only `login/logout` and Claude-only `add-json`, `add-from-claude-desktop`, `serve`,
    `reset-project-choices` remain backend-specific (not promoted to universal v1).
- Gate each operation behind a stable universal capability id:
  - `agent_api.tools.mcp.list.v1`
  - `agent_api.tools.mcp.get.v1`
  - `agent_api.tools.mcp.add.v1`
  - `agent_api.tools.mcp.remove.v1`
- Enforce **safe-by-default** posture:
  - These commands read/write persistent tool configuration and may surface secrets.
  - Built-in backends MUST NOT advertise write capabilities (`add/remove`) unless explicitly enabled
    via the public backend config fields:
    - `agent_api::backends::codex::CodexBackendConfig.allow_mcp_write: bool` (default `false`)
    - `agent_api::backends::claude_code::ClaudeCodeBackendConfig.allow_mcp_write: bool` (default `false`)
  - Built-in backends SHOULD support isolated homes (e.g., `codex_home`, `claude_home`) so
    programmatic usage does not mutate user state by default.

### Why

- Orchestrators need a backend-neutral way to ensure tool servers are present/removed before a run.
- Modeling this as `AgentWrapperRunRequest.extensions` is wrong: MCP management is a CLI subcommand,
  not a run-time extension knob.
- A minimal common surface avoids prematurely standardizing backend-specific MCP features while
  still enabling cross-agent automation.

## Problem / Context

Both primary backends expose MCP management subcommands:

- Claude Code: `claude mcp {add,get,list,remove}` (+ backend-specific extras)
- Codex: `codex mcp {add,get,list,remove,login,logout}`

Our wrapper crates (`crates/codex`, `crates/claude_code`) already expose these subcommands, but
consumers of the **Universal Agent API** currently have no unified way to:

1) list configured MCP servers,
2) inspect a server config,
3) add a new server, or
4) remove a server,

without writing backend-specific glue code.

## Goals

- Provide a minimal, stable, capability-gated MCP management surface in `agent_api`.
- Keep the surface bounded and explicit (no “arbitrary argv pass-through”).
- Preserve backend autonomy:
  - universalize only what is truly common,
  - keep backend-specific MCP features behind backend-scoped capabilities and APIs.
- Maintain Universal Agent API posture:
  - fail-closed on unsupported operations,
  - explicit dangerous operations,
  - bounded outputs (avoid unbounded memory use).

## Non-Goals

- Standardizing a universal MCP server config schema in v1.
- Guaranteeing output format parity (`--json` vs text) across backends.
- Universalizing authentication flows:
  - Codex `mcp login/logout` stays backend-specific.
- Universalizing Claude’s MCP extras (`add-json`, `serve`, etc.).
- Modeling MCP management as run extensions under `AgentWrapperRunRequest.extensions`.

## Proposed API Shape (draft)

Define a dedicated MCP management API surface in `crates/agent_api` that is **separate** from
`AgentWrapperRunRequest`:

- Primary entrypoint: `AgentWrapperGateway` convenience methods.
- Backend hook: add default methods to `AgentWrapperBackend` (non-breaking; default returns
  `UnsupportedCapability`), mirroring the `run_control` pattern.

The API is pinned in:
- `docs/specs/universal-agent-api/mcp-management-spec.md` (draft, normative language)

## Capability Gating (draft)

- Each MCP management operation is gated by a distinct capability id.
- If a caller invokes an operation the backend does not advertise, it MUST fail-closed with:
  `AgentWrapperError::UnsupportedCapability { agent_kind, capability }`.
- If a caller targets an unregistered backend, `AgentWrapperGateway` MUST return:
  `AgentWrapperError::UnknownBackend`.

## Safety / Security Posture (draft)

MCP management commands are **operator-grade**: they interact with persistent configuration and may
read/write sensitive values (env vars, headers, tokens).

Requirements:

- Backends MUST NOT mutate the parent process environment.
- Backends SHOULD support isolated “home” layouts so test/automation flows can be safely isolated.
- Outputs MUST be bounded (stdout/stderr truncation) and MUST NOT be automatically emitted as run
  events (no accidental logging via the run event pipeline).
- Write operations (`add/remove`) MUST require explicit enablement via the public backend config
  fields `CodexBackendConfig.allow_mcp_write` / `ClaudeCodeBackendConfig.allow_mcp_write`
  (default `false`); otherwise the backend MUST NOT advertise the write capabilities.

## Backend Mapping (draft)

Universal operations map to backend subcommands as follows (transport-specific; see the canonical MCP spec for the pinned argv
construction contract):

| Universal op | Claude Code | Codex |
| --- | --- | --- |
| list | `claude mcp list` | `codex mcp list --json` |
| get | `claude mcp get <name>` | `codex mcp get --json <name>` |
| add (Stdio) | `claude mcp add --transport stdio [--env KEY=VALUE]* <name> <commandOrUrl> [args...]` | `codex mcp add <name> [--env KEY=VALUE]* -- <argv...>` |
| add (Url) | `claude mcp add --transport http <name> <url>` | `codex mcp add <name> --url <url> [--bearer-token-env-var <ENV_VAR>]` |
| remove | `claude mcp remove <name>` | `codex mcp remove <name>` |

Notes:

- The universal surface standardizes only request *intent* (add/get/list/remove), not every flag.
- Where upstream CLIs differ (e.g., transport flags, scope flags, JSON output), v1 does not force a
  least-common-denominator schema; backend-specific enhancements remain backend-scoped.
- The pinned argv construction contract for built-in backends (including Codex `--json` usage and Claude `--transport`
  usage) lives in the canonical MCP spec:
  - `docs/specs/universal-agent-api/mcp-management-spec.md` → “Built-in backend behavior” → “Built-in backend mappings (pinned)”
- For v1, Claude URL `bearer_token_env_var` mapping is rejected (fail closed); see the pinned MCP spec.

## Alternatives Considered

1) **Run extensions (`AgentWrapperRunRequest.extensions`)**
   - Rejected: MCP management is not a run option; it is a different command tree and lifecycle.

2) **Arbitrary argv pass-through**
   - Rejected: violates bounded/validated API posture and makes capability gating meaningless.

3) **Universal structured MCP config schema**
   - Deferred: requires a stable cross-backend representation we do not have yet.

## Validation Plan (draft)

- Add unit tests validating:
  - capability gating for each operation,
  - fail-closed behavior on unsupported operations,
  - request validation (non-empty server names, required URL/command fields).
- Add backend harness integration tests (default hermetic fake binaries; live smoke tests against real binaries are opt-in) that:
  - run MCP list/get/add/remove against an isolated home directory,
  - do not require network access.
