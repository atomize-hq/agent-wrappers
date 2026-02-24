# SEAM-1 — Cancellation contract (CA-C01)

## Seam Brief (Restated)

- **Seam ID**: SEAM-1
- **Name**: Cancellation contract (CA-C01)
- **Goal / value**: Add an explicit cancellation control-plane primitive for `agent_api` runs so orchestrators can reliably request backend termination independent of drop semantics and independent of consumer-side timeouts.
- **Type**: platform (public API + normative semantics)
- **Scope**
  - In:
    - Pin the public Rust API surface for explicit cancellation (`run_control(...)`, `AgentWrapperRunControl`, `AgentWrapperCancelHandle`).
    - Pin exact capability gating semantics for explicit cancellation (`agent_api.control.cancel.v1`).
    - Pin exact completion outcome semantics for explicit cancellation (pinned error shape and message).
    - Update canonical contracts/specs under `docs/specs/universal-agent-api/**` to match the pack contract.
  - Out:
    - Harness cancellation signal propagation and drain-on-drop preservation (SEAM-2 / CA-C02).
    - Backend process termination behavior for built-in backends (SEAM-3 / CA-C03).
    - Harness-level/integration tests pinning runtime behavior (SEAM-4).
- **Touch surface**:
  - `crates/agent_api/src/lib.rs` (public Rust API + trait/gateway surface)
  - `docs/specs/universal-agent-api/contract.md` (canonical Rust surface)
  - `docs/specs/universal-agent-api/run-protocol-spec.md` (normative cancellation semantics)
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` (capability id meaning)
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` (pack-local contract)
- **Verification**:
  - Specs: all normative docs agree on capability id, error shape, and pinned strings.
  - Code: `crates/agent_api` compiles with the new surface; `AgentWrapperGateway::run(...)` remains unchanged.
  - Minimal contract tests cover `run_control(...)` error behavior for unknown backend and unsupported capability.
- **Threading constraints**
  - Upstream blockers: none (critical-path start)
  - Downstream blocked seams:
    - SEAM-2 (harness wiring) depends on the public types + semantics being pinned.
    - SEAM-3 (backend termination) depends on the control-plane surface existing.
    - SEAM-4 (tests) depends on SEAM-2/3 runtime wiring, which depends on SEAM-1.
  - Contracts produced (owned):
    - CA-C01 — Public cancellation surface
  - Contracts consumed:
    - (none)

## Slice index

- `S1` → `slice-1-canonical-contracts.md`: Make CA-C01 fully concrete and consistent across pack + canonical specs.
- `S2` → `slice-2-agent-api-surface.md`: Implement the public Rust surface in `crates/agent_api` (scaffold only; wiring owned by SEAM-2/3).

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `CA-C01` (SEAM-1): public cancellation surface + semantics
    - Lives in:
      - Pack: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`
      - Canon: `docs/specs/universal-agent-api/contract.md`, `docs/specs/universal-agent-api/run-protocol-spec.md`, `docs/specs/universal-agent-api/capabilities-schema-spec.md`
    - Produced by: `S1` (docs) and `S2` (code surface)
- **Contracts consumed**:
  - (none)
- **Dependency edges honored**:
  - `SEAM-1 (contract)` → `SEAM-2 (harness wiring)` → `SEAM-3 (backend hooks)` → `SEAM-4 (tests)`: this plan delivers the SEAM-1 outputs first, and does not reach into SEAM-2/3/4 touch surfaces.
- **Parallelization notes**:
  - What can proceed now:
    - SEAM-1 S1 can proceed independently (docs alignment).
    - SEAM-1 S2 can proceed independently once S1 pins exact signatures/strings.
  - What must wait:
    - SEAM-2/3 runtime cancellation wiring must wait on S2 (public types + gateway/trait surface).
    - SEAM-4 integration tests must wait on SEAM-2/3.

