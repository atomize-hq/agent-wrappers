# Threaded Seam Decomposition — SEAM-1 MCP management contract + `agent_api` surface

Pack: `docs/project_management/packs/active/agent-api-mcp-management/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-mcp-management/seam-1-mcp-management-contract.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-mcp-management/threading.md`
- Canonical spec (normative once approved): `docs/specs/universal-agent-api/mcp-management-spec.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-1
- **Name**: MCP management contract + `agent_api::mcp` surface
- **Goal / value**: Give orchestrators a backend-neutral, typed, capability-gated API to manage MCP server configs without depending on wrapper-specific crates.
- **Type**: integration (contract-definition)
- **Scope**
  - In:
    - Implement public `agent_api::mcp` module + pinned type shapes from `docs/specs/universal-agent-api/mcp-management-spec.md`.
    - Add gateway entrypoints: `AgentWrapperGateway::{mcp_list,mcp_get,mcp_add,mcp_remove}` with deterministic error ordering.
    - Add default backend hooks: `AgentWrapperBackend::{mcp_list,mcp_get,mcp_add,mcp_remove}` (default: `UnsupportedCapability`).
    - Provide SEAM-1–owned helpers for:
      - request validation (name + transport), and
      - output bounds + truncation algorithm,
      so SEAM-3/4 can reuse invariants without drift.
  - Out:
    - Backend enablement + safe default advertising + isolated home wiring (SEAM-2).
    - Built-in backend argv mappings + process execution wiring (SEAM-3/4).
    - Cross-backend mapping tests + conformance harness (SEAM-5).
- **Touch surface**
  - `docs/specs/universal-agent-api/mcp-management-spec.md` (canonical contract)
  - `crates/agent_api/src/lib.rs` (gateway + trait surface)
  - `crates/agent_api/src/mcp.rs` (new public module)
  - `crates/agent_api/src/bounds.rs` (reuse/extend for MCP output bounds, if needed)
- **Verification**
  - Unit tests for request validation + safe/redacted `InvalidRequest` messages.
  - Unit tests pinning gateway error ordering + capability gating.
  - Unit tests pinning output bounds + truncation algorithm (UTF-8 safe).
- **Threading constraints**
  - Upstream blockers: none
  - Downstream blocked seams: SEAM-2, SEAM-3, SEAM-4, SEAM-5
  - Contracts produced (owned): MM-C01, MM-C02, MM-C03, MM-C04, MM-C05
  - Contracts consumed: none (within this pack)

## Slicing Strategy

**Contract-first / dependency-first**: SEAM-1 blocks SEAM-2/3/4/5. Land the typed surface + capability ids + gateway/hooks first, then pin shared validation and output-bounds helpers so backend mapping seams can proceed without duplicating invariants.

## Vertical Slices

- **S1 — Public surface + capability-gated gateway/hooks**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-1-mcp-management-contract/slice-1-public-surface.md`
- **S2 — Shared request validation (safe/redacted) + validate-before-hook**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-1-mcp-management-contract/slice-2-request-validation.md`
- **S3 — Output bounds helper + truncation algorithm tests**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-1-mcp-management-contract/slice-3-output-bounds.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `MM-C01 — MCP management capability ids (v1)`: capability id strings used by gateway + backends (produced by S1).
  - `MM-C02 — Non-run command boundary`: dedicated API surface (`agent_api::mcp` + gateway/hooks) outside the run event pipeline (produced by S1; sanity-checked across S1–S3).
  - `MM-C03 — Process context contract`: request context type + semantics surface (`AgentWrapperMcpCommandContext`) (produced by S1; precedence implemented in SEAM-2/3/4).
  - `MM-C04 — Output bounds contract`: bounded stdout/stderr + deterministic truncation marker + flags (helper produced by S3; executed by SEAM-3/4).
  - `MM-C05 — Add transport typing (no argv pass-through)`: typed add transport enum + validation helper entrypoint (type produced by S1; validation produced by S2).
- **Contracts consumed**:
  - None for SEAM-1 (must remain unblocked by SEAM-2/3/4).
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: S1 provides capability ids + hook surface that enablement/advertising will reference.
  - `SEAM-1 blocks SEAM-3`: S1 provides types + gateway/hooks; S2 (validation) + S3 (output bounds) provide invariants Codex mapping must reuse.
  - `SEAM-1 blocks SEAM-4`: S1 provides types + gateway/hooks; S2 (validation) + S3 (output bounds) provide invariants Claude mapping must reuse.
  - `SEAM-1 blocks SEAM-5`: S2/S3 pin deterministic pure behaviors needed for stable unit/integration tests.
- **Parallelization notes**:
  - What can proceed now:
    - Land S1 first to unblock SEAM-2/3/4 compile-time integration against `agent_api::mcp`.
    - SEAM-5 can draft tests that compile-gate the surface once S1 exists.
  - What must wait:
    - SEAM-3/4 mapping work should not finalize semantics until S2 (validation) and S3 (output bounds helper) are available (avoid semantic drift).

## Integration suggestions (explicitly out-of-scope for SEAM-1 tasking)

- Once S1–S3 land, run `make preflight` as WS-INT and proceed per `threading.md` critical path.
- Prefer reusing SEAM-1 validation + output-bounds helpers in SEAM-3/4, rather than duplicating logic in each backend mapping.
