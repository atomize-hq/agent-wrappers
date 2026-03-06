# Threaded Seam Decomposition — SEAM-4 Claude Code backend mapping

Pack: `docs/project_management/packs/active/agent-api-mcp-management/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-mcp-management/seam-4-claude-code-mapping.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-mcp-management/threading.md`
- Canonical spec (normative once approved): `docs/specs/universal-agent-api/mcp-management-spec.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-4
- **Name**: Claude Code MCP management mapping
- **Goal / value**: Implement universal MCP management operations for the **built-in Claude Code backend** by mapping the
  universal requests to pinned `claude mcp add/get/list/remove` argv, while enforcing:
  - non-run boundary (MM-C02),
  - process context precedence (MM-C03),
  - bounded stdout/stderr capture + truncation (MM-C04),
  - typed add transports (MM-C05),
  - safe-by-default write posture and isolation (MM-C06/MM-C07),
  - pinned target availability + fail-closed behavior on unsupported targets (MM-C09).
- **Type**: platform (backend mapping)
- **Scope**
  - In:
    - Implement `AgentWrapperBackend::{mcp_list,mcp_get,mcp_add,mcp_remove}` for `ClaudeCodeBackend`.
    - Pin argv construction to `docs/specs/universal-agent-api/mcp-management-spec.md`
      (“Built-in backend mappings (pinned)”):
      - `list` → `claude mcp list`
      - `get` → `claude mcp get <name>` (**win32-x64 only** per `cli_manifests/claude_code/current.json`)
      - `remove` → `claude mcp remove <name>` (**win32-x64 only**; additionally gated by write enablement in SEAM-2)
      - `add` (**win32-x64 only**; additionally gated by write enablement in SEAM-2):
        - `Stdio` → `claude mcp add --transport stdio [--env KEY=VALUE]* <name> <command> [args...]`
        - `Url`:
          - when `bearer_token_env_var == None` → `claude mcp add --transport http <name> <url>`
          - when `bearer_token_env_var == Some(_)` → reject as `InvalidRequest` (pinned; fail closed).
    - Enforce pinned command execution semantics:
      - return `Ok(output)` even on non-zero exit status once an `ExitStatus` is observed,
      - return `Err(Backend)` only for spawn/wait/timeout/capture failures or manifest/runtime conflicts (drift),
      - do not mutate advertised capabilities at runtime (fail closed; remediation is updating pinned manifests).
    - Ensure spawned CLI honors `request.context.{working_dir,timeout,env}` without mutating the parent env.
  - Out:
    - Universal type surface + gateway/hooks + shared validation/bounds helpers (SEAM-1).
    - Capability advertising + `allow_mcp_write` + isolated homes wiring (SEAM-2).
    - Cross-backend hermetic fake-binary integration tests (SEAM-5).
- **Touch surface**:
  - `crates/agent_api/src/backends/claude_code.rs` (backend hook implementation; minimal wiring)
  - New Claude-only helper module(s) (recommended to reduce conflicts with SEAM-3), e.g.:
    - `crates/agent_api/src/backends/claude_code/mcp_management.rs`
- **Verification**:
  - Unit tests for:
    - argv composition for `list/get/add/remove` (pure),
    - `Url.bearer_token_env_var` rejection for Claude (pure),
    - capability gating behavior (fail closed),
    - process context precedence + env collision rules (pure),
    - bounded capture primitive behavior (no unbounded buffering),
    - manifest/runtime drift classifier (conservative).
  - Hermetic fake-binary tests + isolated-home assertions are owned by SEAM-5.
- **Threading constraints**
  - Upstream blockers: SEAM-1, SEAM-2
  - Downstream blocked seams: SEAM-5
  - Contracts produced (owned): MM-C09
  - Contracts consumed: MM-C01, MM-C02, MM-C03, MM-C04, MM-C05, MM-C06, MM-C07

## Slicing Strategy

**Dependency-first / value-first** within the seam:

1) Land a bounded “Claude MCP command runner” + `list/get` mapping first (read ops unblock early verification, and `get`
   remains safely fail-closed off-target via advertising).
2) Add `add/remove` mapping second, explicitly write-gated and isolation-aware, including the pinned `Url` bearer-token
   rejection behavior.

## Vertical Slices

- **S1 — Read ops (`list/get`) mapping + bounded exec runner**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-4-claude-code-mapping/slice-1-read-ops.md`
- **S2 — Write ops (`add/remove`) mapping + typed transports + write gating**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-4-claude-code-mapping/slice-2-write-ops.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `MM-C09 — Claude MCP mapping contract`:
    - Definition: pinned argv mapping + target availability rules for Claude in
      `docs/specs/universal-agent-api/mcp-management-spec.md` (“Built-in backend behavior”).
    - Where it lives: implemented in `crates/agent_api/src/backends/claude_code.rs`
      (+ a Claude-only helper module such as `crates/agent_api/src/backends/claude_code/mcp_management.rs`).
    - Produced by: S1 (list/get) + S2 (add/remove + bearer-token rejection) complete the mapping.
- **Contracts consumed**:
  - `MM-C01 — MCP management capability ids (v1)` (SEAM-1): S1/S2 fail-closed gating uses the capability ids.
  - `MM-C02 — Non-run command boundary` (SEAM-1): S1/S2 do not emit MCP stdout/stderr as run events.
  - `MM-C03 — Process context contract` (SEAM-1): S1/S2 compute effective working_dir/timeout/env with pinned precedence.
  - `MM-C04 — Output bounds contract` (SEAM-1): S1 implements bounded streaming capture and applies SEAM-1’s enforcement helper.
  - `MM-C05 — Add transport typing (no argv pass-through)` (SEAM-1): S2 maps typed transports only; no extra-args escape hatch.
  - `MM-C06 — Safe default advertising (write ops)` (SEAM-2): S2 write hooks fail-closed when unadvertised / disabled.
  - `MM-C07 — Isolated home support` (SEAM-2): S1/S2 honor `ClaudeCodeBackendConfig.claude_home` injection, while allowing
    request env overrides to win (pinned).
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-4`: S1/S2 assume the final request types + validation helper + output enforcement helper.
  - `SEAM-2 blocks SEAM-4`: S2 assumes `allow_mcp_write` + isolated-home config exist and capability advertising is authoritative.
  - `SEAM-4 blocks SEAM-5`: S1/S2 deliver the concrete Claude mapping that tests will pin with fake binaries.
- **Parallelization notes**:
  - What can proceed now:
    - As soon as SEAM-1 + SEAM-2 land, WS-CLAUDE can implement S1 without coordination with WS-CODEX/WS-TESTS.
  - What must wait:
    - S2 should wait for SEAM-2 write enablement + isolated homes wiring to land (avoid rework).
    - WS-TESTS should not finalize Claude mapping assertions until S1/S2 land (avoid duplication / drift).

## Integration suggestions (explicitly out-of-scope for SEAM-4 tasking)

- Once S1/S2 land, WS-TESTS can pin Claude mapping behavior using hermetic fake `claude` binaries and isolated homes.
  Tests must be target-aware for the pinned `win32-x64` availability of `get/add/remove` (fail closed on other targets).
- After SEAM-5 lands, WS-INT should run `make preflight` per `threading.md`.

