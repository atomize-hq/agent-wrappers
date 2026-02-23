# S1 — Pin `BH-C01` adapter contract as code

- **User/system value**: Unblocks downstream seams by freezing the internal “identity + supported extension keys + spawn + typed mapping + completion extraction” contract shape.
- **Scope (in/out)**:
  - In:
    - Define `BH-C01 backend harness adapter interface` as internal Rust API.
    - Define minimal supporting types/aliases needed to express the interface.
    - Define explicit error-mapping boundaries (redaction points) as part of the interface.
  - Out:
    - Implementing canonical normalization/validation policy (`BH-C02`, SEAM-2).
    - Implementing streaming pump + drain-on-drop semantics (`BH-C04`, SEAM-3).
    - Implementing DR-0012 completion gating integration (`BH-C05`, SEAM-4).
    - Migrating real backends to the harness (SEAM-5).
- **Acceptance criteria**:
  - `BH-C01` exists and is `pub(crate)` in `crates/agent_api/src/backend_harness.rs`.
  - The interface covers:
    - backend identity/kind,
    - supported extension keys surface + backend-specific validation hook surface,
    - spawn returning `(typed stream, completion future)` (or equivalent),
    - typed-event → `AgentWrapperEvent` mapping hook,
    - explicit backend error → `AgentWrapperError` mapping hook(s) at spawn/stream/completion boundaries.
  - Clean build under `--features codex`, `--features claude_code`, and combined.
- **Dependencies**: none.
- **Verification**:
  - `cargo check -p agent_api --features codex`
  - `cargo check -p agent_api --features claude_code`
  - `cargo check -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S1.T1 — Define `BH-C01` interface + supporting types

- **Outcome**: A minimal adapter contract (trait or closure bundle) and spawn/result types that can represent “typed stream + typed completion + mapping”.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backend_harness.rs` (new)
  - Output (wiring): `crates/agent_api/src/lib.rs` (`mod backend_harness;` + `pub(crate)` re-exports if needed)
- **Implementation notes**:
  - Keep everything `pub(crate)` and co-located for auditability.
  - Prefer referencing existing public types (no new public API):
    - `AgentWrapperKind`, `AgentWrapperCapabilities`, `AgentWrapperRunRequest`,
    - `AgentWrapperEvent`, `AgentWrapperCompletion`, `AgentWrapperError`.
  - Represent spawn output explicitly as a `(Stream<Item = Result<TypedEvent, BackendErr>>, Future<Output = Result<TypedCompletion, BackendErr>>)`-like shape with `Send` bounds.
- **Acceptance criteria**:
  - The contract is small enough to review quickly (avoid generic “framework” abstractions).
  - No lifetime gymnastics needed for a backend adapter to implement it.
- **Test notes**: exercised by Slice S2 toy adapter smoke tests.
- **Risk/rollback notes**: internal-only; can be iterated without breaking public API.

Checklist:
- Implement: `BH-C01` trait/struct + minimal spawn types/aliases.
- Test: compile the `agent_api` crate with all relevant feature flags.
- Validate: `make clippy` (warnings are errors) on the workspace.
- Cleanup: ensure the module is clearly internal (no `pub` exports).

#### S1.T2 — Define supported extension keys + backend-specific validation hook surfaces (no enforcement yet)

- **Outcome**: The harness can ask a backend “what extension keys do you support?” and can call a backend-provided validator for backend-specific extension payload semantics.
- **Inputs/outputs**:
  - Output: additions in `crates/agent_api/src/backend_harness.rs`.
- **Implementation notes**:
  - Include a supported-extension-keys accessor as part of the adapter contract.
  - Include a backend-specific validator hook surface that can reject malformed backend-specific payloads.
  - Do **not** implement unknown-key rejection logic here; enforcement/policy is `BH-C02` (SEAM-2).
- **Acceptance criteria**:
  - Downstream seams can implement fail-closed validation without each backend re-implementing allowlists.
  - The boundary between “backend-specific validation” (this seam) and “unknown-key rejection” (SEAM-2) is explicit in docs/comments.
- **Test notes**: toy adapter provides a small allowlist + validator hook that is invoked pre-spawn.
- **Risk/rollback notes**: keep hook minimal; avoid overfitting to current Codex/Claude extension sets.

Checklist:
- Implement: `supported_extension_keys()` + `validate_backend_request()` hook (names TBD).
- Test: call ordering is possible (validate-before-spawn) in the harness lifecycle.
- Validate: no policy logic creeps in (unknown-key rejection stays for SEAM-2).
- Cleanup: document the ownership split (`BH-C02` vs backend-specific validation).

#### S1.T3 — Define canonical error-mapping points (redaction boundary)

- **Outcome**: The contract has explicit hooks for mapping backend-specific errors into `AgentWrapperError` (or bounded/redacted messages) at spawn/stream/completion boundaries.
- **Inputs/outputs**:
  - Output: additions in `crates/agent_api/src/backend_harness.rs`.
- **Implementation notes**:
  - Prefer a single mapping surface with phase context (spawn/stream/completion) rather than scattered ad-hoc conversions.
  - Document intent: prevent leaking raw backend lines or internal error detail into universal envelope semantics.
- **Acceptance criteria**:
  - The harness has a canonical way to map backend failures; downstream seams do not introduce divergent error formatting.
  - Smoke tests can invoke the mapper in at least one boundary.
- **Test notes**: toy adapter returns a sentinel backend error; mapping produces stable `AgentWrapperError::Backend { message: ... }`.
- **Risk/rollback notes**: internal-only; can be refined without public API impact.

Checklist:
- Implement: error mapping hook(s) + optional phase enum.
- Test: a simulated spawn failure maps deterministically.
- Validate: no `Debug` dumps of backend errors are surfaced by default.
- Cleanup: keep the mapping API small and explicit.

