# S2 — Conformance/viability proof (toy adapter smoke + compile gates)

- **User/system value**: Proves `BH-C01` is implementable and sufficient for downstream seams’ expectations (stream + completion + mapping shape), without touching real backends (SEAM-5).
- **Scope (in/out)**:
  - In:
    - Add a `#[cfg(test)]` toy adapter implementing `BH-C01`.
    - Add smoke tests that drive a small typed stream + completion future and map events into `AgentWrapperEvent`.
  - Out:
    - Refactors to `crates/agent_api/src/backends/codex.rs` and `crates/agent_api/src/backends/claude_code.rs` (SEAM-5).
    - Any enforcement/policy behavior (SEAM-2/3/4).
- **Acceptance criteria**:
  - Toy adapter compiles under relevant feature combos and demonstrates the `BH-C01` “spawn → typed stream/completion → mapping → universal events” shape.
  - Tests are narrowly scoped to contract viability (no re-implementation of pump/gating semantics).
- **Dependencies**:
  - Contract produced by S1: `BH-C01 backend harness adapter interface`.
- **Verification**:
  - `cargo test -p agent_api --features codex`
  - `cargo test -p agent_api --features claude_code`
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S2.T1 — Add contract smoke tests (“toy backend adapter”)

- **Outcome**: A minimal test module proving the contract can be implemented without awkward lifetimes or missing `Send` bounds.
- **Inputs/outputs**:
  - Output: test module co-located with the contract (e.g., `crates/agent_api/src/backend_harness.rs` under `#[cfg(test)]`), or a dedicated internal test module in `crates/agent_api/src/`.
- **Implementation notes**:
  - Keep “toy typed event” extremely small (1–2 variants).
  - Demonstrate mapping to `AgentWrapperEvent` with deterministic values (`agent_kind`, `kind`, and either `text` or `message`).
  - Ensure the spawned stream + completion future types satisfy the trait bounds expected for real adapters.
- **Acceptance criteria**:
  - Tests compile and run with no clippy warnings.
  - The contract remains auditable (tests should not force additional abstraction layers).
- **Test notes**:
  - Use a short stream (2 items) and a deterministic completion future.
  - Include one failure-path smoke (e.g., spawn failure mapped via S1.T3’s error hook).
- **Risk/rollback notes**: tests-only; safe to revise as the contract stabilizes.

Checklist:
- Implement: toy adapter + toy typed event/completion types.
- Test: success path (events then completion) + one failure path.
- Validate: feature-flag matrix compilation (codex / claude_code / both).
- Cleanup: keep tests minimal and contract-focused.

## Notes for downstream seams (non-tasking)

- SEAM-3 will consume the pinned “typed stream + completion future + mapping hook” shape.
- SEAM-4 will consume where the harness constructs a run handle and where completion is integrated.
- SEAM-5 will migrate real backends; do not modify those files in SEAM-1 work.

