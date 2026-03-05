# Threaded Seam Decomposition — SEAM-3 Codex backend mapping

Pack: `docs/project_management/packs/active/agent-api-mcp-management/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-mcp-management/seam-3-codex-mapping.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-mcp-management/threading.md`
- Canonical spec (normative once approved): `docs/specs/universal-agent-api/mcp-management-spec.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-3
- **Name**: Codex MCP management mapping
- **Goal / value**: Implement universal MCP management operations for the **built-in Codex backend** by mapping the
  universal requests to pinned `codex mcp add/get/list/remove` argv, while enforcing:
  - process context precedence (MM-C03),
  - bounded stdout/stderr capture + truncation (MM-C04),
  - safe-by-default write posture and isolation (MM-C06/MM-C07),
  - non-run boundary (MM-C02).
- **Type**: platform (backend mapping)
- **Scope**
  - In:
    - Implement `AgentWrapperBackend::{mcp_list,mcp_get,mcp_add,mcp_remove}` for `CodexBackend`.
    - Pin argv construction to `docs/specs/universal-agent-api/mcp-management-spec.md` (“Built-in backend mappings (pinned)”):
      - `list` → `codex mcp list --json`
      - `get` → `codex mcp get --json <name>`
      - `remove` → `codex mcp remove <name>`
      - `add`:
        - `Stdio` → `codex mcp add <name> [--env KEY=VALUE]* -- <argv...>`
        - `Url` → `codex mcp add <name> --url <url> [--bearer-token-env-var <ENV_VAR>]`
    - Enforce command execution + error rules (v1, pinned):
      - `Ok(output)` even when the subprocess exits non-zero,
      - `Err(Backend)` only for spawn/wait/timeout/capture failures and manifest/runtime conflicts.
  - Out:
    - Universal type surface + gateway/hooks + shared validation/bounds helpers (SEAM-1).
    - Capability advertising + `allow_mcp_write` + isolated homes wiring (SEAM-2).
    - Cross-backend hermetic fake-binary integration tests (SEAM-5).
- **Touch surface**:
  - `crates/agent_api/src/backends/codex.rs` (backend hook implementation; minimal wiring)
  - New Codex-only helper module(s) (recommended to reduce conflicts with SEAM-2/4), e.g.:
    - `crates/agent_api/src/backends/codex/mcp_management.rs`
- **Verification**:
  - Unit tests for:
    - argv composition for `list/get/add/remove` (pure),
    - capability gating behavior (fail closed),
    - process context precedence + env collision rules (pure),
    - bounded capture primitive behavior (no unbounded buffering).
  - Hermetic fake-binary tests + isolated-home assertions are owned by SEAM-5.
- **Threading constraints**
  - Upstream blockers: SEAM-1, SEAM-2
  - Downstream blocked seams: SEAM-5
  - Contracts produced (owned): MM-C08
  - Contracts consumed: MM-C01, MM-C02, MM-C03, MM-C04, MM-C05, MM-C06, MM-C07

## Slicing Strategy

**Dependency-first / value-first** within the seam:

1) Land a bounded “Codex MCP command runner” + `list/get` mapping first (read ops unblock early verification).
2) Add `add/remove` mapping second, explicitly write-gated and isolation-aware.

## Vertical Slices

- **S1 — Read ops (`list/get`) mapping + bounded exec runner**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-3-codex-mapping/slice-1-read-ops.md`
- **S2 — Write ops (`add/remove`) mapping + typed transports + write gating**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-3-codex-mapping/slice-2-write-ops.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `MM-C08 — Codex MCP mapping contract`:
    - Definition: pinned argv mapping for Codex in `docs/specs/universal-agent-api/mcp-management-spec.md` (“Built-in backend mappings (pinned)”).
    - Where it lives: implemented in `crates/agent_api/src/backends/codex.rs` (+ a Codex-only helper module).
    - Produced by: S1 (list/get) + S2 (add/remove) complete the mapping.
- **Contracts consumed**:
  - `MM-C01 — MCP management capability ids (v1)` (SEAM-1): S1/S2 fail-closed gating uses the capability ids.
  - `MM-C02 — Non-run command boundary` (SEAM-1): S1/S2 do not emit MCP stdout/stderr as run events.
  - `MM-C03 — Process context contract` (SEAM-1): S1/S2 compute effective working_dir/timeout/env with pinned precedence.
  - `MM-C04 — Output bounds contract` (SEAM-1): S1 implements bounded streaming capture + calls SEAM-1 enforcement helper.
  - `MM-C05 — Add transport typing (no argv pass-through)` (SEAM-1): S2 maps typed transports only; no extra-args escape hatch.
  - `MM-C06 — Safe default advertising (write ops)` (SEAM-2): S2 write hooks fail-closed when unadvertised / disabled.
  - `MM-C07 — Isolated home support` (SEAM-2): S1/S2 honor `CodexBackendConfig.codex_home` injection, while allowing
    request env overrides to win (pinned).
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-3`: S1/S2 assume the final request types + validation helper + output enforcement helper.
  - `SEAM-2 blocks SEAM-3`: S2 assumes `allow_mcp_write` + isolated-home config exist and capability advertising is authoritative.
  - `SEAM-3 blocks SEAM-5`: S1/S2 deliver the concrete Codex mapping that tests will pin with fake binaries.
- **Parallelization notes**:
  - What can proceed now:
    - As soon as SEAM-1 + SEAM-2 land, WS-CODEX can implement S1 without coordination with WS-CLAUDE/WS-TESTS.
  - What must wait:
    - S2 should wait for SEAM-2 write enablement + isolated homes wiring to land (avoid rework).
    - WS-TESTS should not finalize Codex mapping assertions until S1/S2 land (avoid duplication / drift).

## Integration suggestions (explicitly out-of-scope for SEAM-3 tasking)

- Once S1/S2 land, WS-TESTS can pin Codex mapping behavior using hermetic fake `codex` binaries and isolated homes, then
  WS-INT should run `make preflight` per `threading.md`.

