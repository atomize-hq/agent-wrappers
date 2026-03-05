# Threaded Seam Decomposition — SEAM-2 Backend enablement + safe default advertising

Pack: `docs/project_management/packs/active/agent-api-mcp-management/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-mcp-management/seam-2-backend-enablement.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-mcp-management/threading.md`
- Canonical spec (normative once approved): `docs/specs/universal-agent-api/mcp-management-spec.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-2
- **Name**: Backend enablement + safe default advertising (write ops) + isolated homes
- **Goal / value**: Ensure MCP management is safe-by-default:
  - built-in backends do not advertise write operations unless explicitly enabled, and
  - automation/tests can run MCP management against isolated homes to avoid mutating user state.
- **Type**: integration (safety / permissions)
- **Scope**
  - In:
    - Define host-provided backend config knobs that control MCP management capability advertising:
      - `allow_mcp_write` (default: `false`) gates `add/remove`.
    - Implement capability advertising logic for built-in backends that follows pinned precedence:
      - upstream target availability (from pinned CLI manifests) → config enablement → advertise.
    - Support isolated homes via backend config (`codex_home` / `claude_home`) so state mutations can be confined to a
      dedicated root.
  - Out:
    - Mapping universal requests to upstream argv for Codex (SEAM-3) and Claude Code (SEAM-4).
    - Cross-backend conformance/integration tests (SEAM-5).
    - Generalized permission/policy engines beyond MCP management.
- **Touch surface**
  - `crates/agent_api/src/backends/codex.rs` (config + advertising)
  - `crates/agent_api/src/backends/claude_code.rs` (config + advertising + isolation wiring)
  - Wrapper builders (only if gaps exist for isolation wiring):
    - `crates/codex/src/builder/mod.rs` (already supports `codex_home`)
    - `crates/claude_code/src/builder/mod.rs` (already supports `claude_home`)
- **Verification**
  - Unit tests pinning:
    - default advertising posture (write ops off by default),
    - `allow_mcp_write` gating,
    - pinned target-availability gating (Claude `win32-x64` only for `get/add/remove`),
    - request env overrides winning over isolated-home env injection (ability to intentionally defeat isolation is pinned).
  - Compile/test under feature matrix:
    - `cargo test -p agent_api --features codex`
    - `cargo test -p agent_api --features claude_code`
    - `cargo test -p agent_api --features codex,claude_code`
- **Threading constraints**
  - Upstream blockers: SEAM-1
  - Downstream blocked seams: SEAM-3, SEAM-4, SEAM-5
  - Contracts produced (owned): MM-C06, MM-C07
  - Contracts consumed: MM-C01, MM-C03 (plus SEAM-1’s non-run boundary + output bounds invariants)

## Slicing Strategy

**Dependency-first / contract-first**: SEAM-2 blocks SEAM-3/4/5. Land enablement + advertising + isolated home plumbing
early so backend mapping seams can implement hooks without re-litigating safety posture.

## Vertical Slices

- **S1 — Safe default advertising + write enablement gating**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-2-backend-enablement/slice-1-safe-advertising.md`
- **S2 — Isolated homes (backend config + wiring)**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-2-backend-enablement/slice-2-isolated-homes.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `MM-C06 — Safe default advertising (write ops)`: `allow_mcp_write` gates `add/remove`, and built-in backends MUST NOT
    advertise write capability ids unless explicitly enabled (produced by S1).
  - `MM-C07 — Isolated home support`: built-in backends support per-backend isolated homes via config:
    `CodexBackendConfig.codex_home` and `ClaudeCodeBackendConfig.claude_home` (produced by S2).
- **Contracts consumed**:
  - `MM-C01 — MCP management capability ids (v1)`: used for capability advertising (S1).
  - `MM-C03 — Process context contract`: request-level env overrides win over backend-provided env (S2 precedence rules).
  - `MM-C02/MM-C04/MM-C05`: respected indirectly (no run events; bounded outputs; typed transports) but implemented in other seams.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: S1/S2 require the final capability ids + request context semantics defined in SEAM-1.
  - `SEAM-2 blocks SEAM-3`: S1 provides write enablement + advertising rules; S2 provides isolated home wiring required by
    Codex mapping for safe write ops.
  - `SEAM-2 blocks SEAM-4`: S1 provides write enablement + advertising rules; S2 provides isolated home wiring required by
    Claude mapping for safe write ops.
  - `SEAM-2 blocks SEAM-5`: tests must pin safe default advertising + isolated home behavior and require stable enablement
    knobs.
- **Parallelization notes**:
  - What can proceed now:
    - Once SEAM-1 lands, S1 can be implemented independently of backend argv mapping code (SEAM-3/4).
    - S2 can be implemented in parallel with S1 (different code paths) but should land before SEAM-3/4 finalize hooks.
  - What must wait:
    - SEAM-3/4 MUST treat S1/S2 as authoritative: no per-backend ad hoc advertising or isolation logic.
    - SEAM-5 conformance tests for advertising + isolation should wait for S1/S2 to land.

## Integration suggestions (explicitly out-of-scope for SEAM-2 tasking)

- After S1/S2 land, SEAM-3/4 should implement MCP hooks using the enablement + isolation knobs here, then WS-TESTS/WS-INT
  can proceed per `threading.md` critical path.
