# Seam map — Universal MCP management commands (add/get/list/remove)

Primary axis: **integration-first (risk-first)** — cross-backend MCP management surface + safety posture.

## Seams

1) **SEAM-1 — MCP management contract + `agent_api` surface**
   - Owns: the universal request/response types, capability ids, gateway entrypoints, backend hooks, and pinned output bounds.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-mcp-management/seam-1-mcp-management-contract.md`
     - implementation in `crates/agent_api` (new `agent_api::mcp` module + trait/gateway hooks)

2) **SEAM-2 — Backend enablement + safe default advertising**
   - Owns: safe-by-default posture for built-in backends (write ops require explicit enablement) and isolated home support.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-mcp-management/seam-2-backend-enablement.md`
     - updates to `crates/agent_api/src/backends/{codex,claude_code}.rs`

3) **SEAM-3 — Codex backend mapping**
   - Owns: mapping the universal MCP requests to `codex mcp add/get/list/remove` with bounded outputs and process context.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-mcp-management/seam-3-codex-mapping.md`
     - updates to `crates/agent_api/src/backends/codex.rs` (+ wrapper surface if needed)

4) **SEAM-4 — Claude Code backend mapping**
   - Owns: mapping the universal MCP requests to `claude mcp add/get/list/remove` with bounded outputs and process context.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-mcp-management/seam-4-claude-code-mapping.md`
     - updates to `crates/agent_api/src/backends/claude_code.rs` (+ wrapper surface if needed)

5) **SEAM-5 — Tests**
   - Owns: regression coverage for capability gating, request validation, output truncation, safe default advertising, and
     backend mapping (with isolated homes).
   - Outputs:
     - `docs/project_management/packs/active/agent-api-mcp-management/seam-5-tests.md`
     - updates/additions to tests under `crates/agent_api/src/**` (and optionally wrapper crates if gaps exist)

