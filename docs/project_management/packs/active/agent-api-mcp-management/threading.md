# Threading — Universal MCP management commands (add/get/list/remove)

This section makes coupling explicit: contracts/interfaces, dependency edges, and sequencing.

## Contract registry

- **MM-C01 — MCP management capability ids (v1)**
  - **Type**: permission
  - **Definition**: operation-specific capability ids (v1):
    - `agent_api.tools.mcp.list.v1`
    - `agent_api.tools.mcp.get.v1`
    - `agent_api.tools.mcp.add.v1`
    - `agent_api.tools.mcp.remove.v1`
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5

- **MM-C02 — Non-run command boundary**
  - **Type**: policy
  - **Definition**: MCP management is a **non-run** API surface and MUST NOT be modeled as run extensions.
    MCP management outputs MUST NOT be emitted as run events (`AgentWrapperEvent`s).
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **MM-C03 — Process context contract**
  - **Type**: config
  - **Definition**: requests support per-command:
    - `working_dir` override (optional)
    - `timeout` override (optional)
    - `env` overrides applied only to spawned backend processes (request keys win; parent env not mutated)
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **MM-C04 — Output bounds contract**
  - **Type**: schema
  - **Definition**: stdout/stderr captured with pinned budgets:
    - `stdout`: 65,536 bytes
    - `stderr`: 65,536 bytes
    If truncated, append suffix `…(truncated)` and set `stdout_truncated`/`stderr_truncated` accordingly (UTF-8 preserved).
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **MM-C05 — Add transport typing (no argv pass-through)**
  - **Type**: API
  - **Definition**: `mcp_add` is expressed as typed transport:
    - `Stdio { command, args, env }` (server launch + env injection)
    - `Url { url, bearer_token_env_var }` (HTTP server config)
    and does not allow generic “extra args” for the management command itself.
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **MM-C06 — Safe default advertising (write ops)**
  - **Type**: permission
  - **Definition**: built-in backends MUST NOT advertise write capabilities (`add/remove`) unless explicitly enabled via backend
    configuration. Read operations may be advertised by default (exact defaults pinned in SEAM-2).
  - **Owner seam**: SEAM-2
  - **Consumers**: SEAM-3/4/5

- **MM-C07 — Isolated home support**
  - **Type**: integration
  - **Definition**: built-in backends SHOULD support isolated homes (e.g., `codex_home`, `claude_home`) so automation/tests can
    manage MCP config under a dedicated state root (no user-state mutation by default).
  - **Owner seam**: SEAM-2
  - **Consumers**: SEAM-3/4/5

- **MM-C08 — Codex MCP mapping contract**
  - **Type**: integration
  - **Owner seam**: SEAM-3
  - **Consumers**: SEAM-5
  - **Definition**: map universal operations to `codex mcp ...` (see `cli_manifests/codex/current.json`):
    - `list` → `codex mcp list` (optionally `--json`, pinned in SEAM-3)
    - `get` → `codex mcp get <name>` (optionally `--json`, pinned in SEAM-3)
    - `add`:
      - `Stdio` → `codex mcp add <name> [--env KEY=VALUE]* -- <command...>`
      - `Url` → `codex mcp add <name> --url <url> [--bearer-token-env-var ENV_VAR]`
    - `remove` → `codex mcp remove <name>`

- **MM-C09 — Claude MCP mapping contract**
  - **Type**: integration
  - **Owner seam**: SEAM-4
  - **Consumers**: SEAM-5
  - **Definition**: map universal operations to `claude mcp ...` (see `cli_manifests/claude_code/current.json`):
    - `list` → `claude mcp list`
    - `get` → `claude mcp get <name>`
    - `add`:
      - `Stdio` → `claude mcp add <name> <command> [args...]` (+ server env mapping where supported)
      - `Url` → `claude mcp add <name> <url>` (+ auth/header mapping pinned in SEAM-4)
    - `remove` → `claude mcp remove <name>`

## Dependency graph (text)

- `SEAM-1 blocks SEAM-2` because: enablement + advertising must be driven by the final capability ids and API surface.
- `SEAM-1 blocks SEAM-3` because: Codex mapping needs the pinned request types, validation rules, and output bounds.
- `SEAM-1 blocks SEAM-4` because: Claude mapping needs the pinned request types, validation rules, and output bounds.
- `SEAM-2 blocks SEAM-3` because: Codex mapping must respect write enablement and isolated-home safety posture.
- `SEAM-2 blocks SEAM-4` because: Claude mapping must respect write enablement and isolated-home safety posture.
- `SEAM-2 blocks SEAM-5` because: tests must pin default advertising posture and isolated home behavior.
- `SEAM-3 blocks SEAM-5` because: tests must pin Codex mapping semantics and request validation.
- `SEAM-4 blocks SEAM-5` because: tests must pin Claude mapping semantics and request validation.

## Critical path

`SEAM-1 (contract)` → `SEAM-2 (enablement)` → `SEAM-3/SEAM-4 (backend mapping)` → `SEAM-5 (tests)`

## Parallelization notes / conflict-safe workstreams

- **WS-SPEC**: SEAM-1 (`agent_api::mcp` public surface + pinned contracts).
- **WS-ENABLEMENT**: SEAM-2 (backend config + capability advertising + isolated homes).
- **WS-CODEX**: SEAM-3 (Codex MCP mapping).
- **WS-CLAUDE**: SEAM-4 (Claude MCP mapping).
- **WS-TESTS**: SEAM-5 (tests; can start with request validation + output truncation harness once SEAM-1 lands).
- **WS-INT (Integration)**: end-to-end validation + `make preflight` once seams land.

## Pinned decisions / resolved threads

- **Non-run boundary**: MCP management stays out of the run event pipeline (MM-C02).
- **No argv pass-through**: typed transports only; no “extra args” escape hatch (MM-C05).
- **Output budgets**: stdout/stderr budgets and truncation marker are pinned (MM-C04).
