# Threaded Seam Decomposition — SEAM-1 Harness contract-definition

Pack: `docs/project_management/packs/active/agent-api-backend-harness/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-backend-harness/seam-1-harness-contract.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-backend-harness/threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-1
- **Name**: Backend harness internal contract-definition
- **Goal / value**: Provide a small, auditable internal interface so each backend adapter is “identity + spawn + map”, and shared invariants are applied consistently by construction.
- **Type**: integration
- **Scope**
  - In:
    - Define the internal harness entrypoint(s) and adapter-facing interface that each backend must implement/provide.
    - Define harness-owned lifecycle touchpoints: request validation hook(s), spawn hook(s), event mapping hook(s), completion extraction.
    - Define how per-backend supported extension keys / capability allowlists are surfaced to the harness.
  - Out:
    - Any change to the public `agent_api` surface.
    - Any change to per-backend typed event models (wrapper-owned).
- **Primary interface (contract)**
  - Produced (owned): `BH-C01 backend harness adapter interface`
- **Key invariants referenced (implemented later)**
  - Unknown extension keys rejected pre-spawn (ADR-0013) — owned by `BH-C02` / SEAM-2.
  - Stream forwarding + drain-on-drop — owned by `BH-C04` / SEAM-3.
  - Completion gating integration — owned by `BH-C05` / SEAM-4.
- **Touch surface (code)**
  - `crates/agent_api/src/backend_harness.rs` (new internal module)
  - `crates/agent_api/src/backends/mod.rs` (only if needed for wiring; avoid in SEAM-1)
  - `crates/agent_api/src/run_handle_gate.rs` (integration boundary; do not change semantics here)
- **Verification**
  - Compile-time: both Codex + Claude adapters are expressible as implementations/usages of the contract.
  - Review-time: interface remains small enough to audit (explicit control flow; no macro indirection).

## Slicing Strategy

**Contract-first / dependency-first**: SEAM-1 blocks SEAM-3, SEAM-4, and SEAM-5, so the first deliverable is a pinned internal interface with smoke-proof that it is implementable.

## Vertical Slices

- **S1 — Pin `BH-C01` adapter contract as code**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`
- **S2 — Conformance/viability proof (toy adapter smoke + compile gates)**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-2-viability-smoke.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `BH-C01 backend harness adapter interface`: internal Rust interface (and minimal supporting types) defined in `crates/agent_api/src/backend_harness.rs` (produced by Slice S1).
- **Contracts consumed**:
  - None for SEAM-1 (must remain unblocked by SEAM-2/3/4).
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-3`: S1 pins the “stream + completion + mapping” contract shape required by the pump.
  - `SEAM-1 blocks SEAM-4`: S1 pins where/how the harness constructs lifecycle boundaries so gating can be integrated later.
  - `SEAM-1 blocks SEAM-5`: S1 provides the interface real backends will adopt, without modifying backends in this seam.
- **Parallelization notes**:
  - What can proceed now: SEAM-1 (WS-A) contract-definition + smoke tests.
  - What must wait: SEAM-2/3/4/5 implementation should start after `BH-C01` lands, per `threading.md` critical path.

## Integration suggestions (explicitly out-of-scope for SEAM-1 tasking)

These are useful risk checks, but they touch SEAM-5 surfaces and should be done as WS-INT/SEAM-5 work (not in this seam’s PRs):
- Spike a non-landing sketch adapting one real backend (`codex` or `claude_code`) to the `BH-C01` shape to confirm the contract fits.

