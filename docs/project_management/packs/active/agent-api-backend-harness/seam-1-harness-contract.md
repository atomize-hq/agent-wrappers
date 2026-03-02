# SEAM-1 — Harness contract-definition

- **Name**: Backend harness internal contract-definition
- **Type**: integration
- **Goal / user value**: Provide a small, auditable internal interface so each backend adapter is “identity + spawn + map”, and shared invariants are applied consistently by construction.
- **Scope**
  - In:
    - Define the internal harness entrypoint and adapter-facing interface that each backend must provide:
      - `BackendHarnessAdapter` (`pub(crate)` trait)
      - `backend_harness::run_harnessed_backend(...) -> Result<AgentWrapperRunHandle, AgentWrapperError>`
      - All in `crates/agent_api/src/backend_harness.rs` (internal-only).
    - Define the harness-owned lifecycle: request validation hook(s), spawn hook(s), event mapping hook(s), and completion extraction.
    - Define how per-backend capabilities/extension allowlists are surfaced to the harness.
  - Out:
    - Any change to the public `agent_api` surface.
    - Any change to per-backend typed event models (those remain wrapper-owned).
- **Primary interfaces (contracts)**
  - Inputs:
    - `AgentWrapperRunRequest` (universal request).
    - Backend-provided “supported extension keys” set + backend-specific validation routine(s).
    - Backend spawn routine returning:
      - a typed event stream (`Stream<Item = Result<TypedEvent, BackendErr>>`), and
      - a completion future producing a typed completion value (or error).
    - Backend mapping routine from typed events → `AgentWrapperEvent` (universal envelope).
  - Outputs:
    - `AgentWrapperRunHandle` with canonical gating and bounded events semantics.
    - Canonical error mapping points (redacted/bounded).
- **Key invariants / rules**:
  - Harness must be able to enforce: fail-closed extension validation, env precedence, timeout wrapping, bounds enforcement, drain-on-drop, and DR-0012 completion gating wiring.
  - Unknown extension keys MUST be rejected before spawn (per ADR-0013).
- **Dependencies**
  - Blocks:
    - `SEAM-3` (streaming pump) — needs the contract’s “stream + completion” shape.
    - `SEAM-4` (gating) — needs the run-handle lifecycle contract.
    - `SEAM-5` (backend adoption) — backends can’t be migrated without the interface.
  - Blocked by:
    - None (this is the first thin-contract seam).
- **Touch surface**:
  - `crates/agent_api/src/` (new internal module, e.g. `backend_harness.rs`)
  - `crates/agent_api/src/backends/mod.rs` (if needed for wiring)
  - `crates/agent_api/src/run_handle_gate.rs` (integration boundary for gating)
- **Verification**:
  - Compile-time: both Codex and Claude adapters can be expressed as implementations/usages of the contract.
  - Review-time: interface is small enough to audit (no macro magic; explicit control flow).
- **Risks / unknowns**
  - Risk: A too-generic abstraction leaks backend-specific behavior into the harness (or vice versa).
  - De-risk plan: define the thinnest possible contract first (“contract-definition item”), then spike a Codex port in a branch to confirm the interface fits.
- **Rollout / safety**:
  - Internal refactor only; use backend adoption + tests (SEAM-5) as the rollout gate.

## Downstream decomposition prompt

Decompose this seam into slices that (1) pin the internal interface, (2) implement a minimal harness skeleton, and (3) demonstrate viability by adapting one backend end-to-end without changing observable behavior.
