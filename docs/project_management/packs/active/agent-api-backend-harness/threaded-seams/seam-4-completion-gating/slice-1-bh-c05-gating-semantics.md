# S1 — Document and pin `BH-C05` gating semantics (DR-0012)

- **User/system value**: Makes the intended DR-0012 gating behavior explicit and auditable, so future harness refactors cannot accidentally change when `AgentWrapperRunHandle.completion` becomes observable relative to stream finality.
- **Scope (in/out)**:
  - In:
    - Document the exact semantics enforced by the canonical gate builder in `run_handle_gate`.
    - Pin terminology (“stream finality”, “consumer drop”, “completion outcome readiness”) so SEAM-3 and SEAM-4 talk about the same thing.
    - Explicitly state the allowed early-completion escape hatch when the consumer drops the events stream.
  - Out:
    - Changing DR-0012 semantics.
    - Changing the public `AgentWrapperRunHandle` API.
    - Implementing the harness orchestration itself (SEAM-1/SEAM-3).
- **Acceptance criteria**:
  - `BH-C05` semantics are written down next to the canonical implementation (`crates/agent_api/src/run_handle_gate.rs`), including:
    - what “finality” means for the universal events stream,
    - what happens when the consumer drops the events stream,
    - what happens when the completion channel drops.
  - The doc explicitly notes the coupling boundary with SEAM-3:
    - SEAM-3 owns when the sender is dropped (stream finality) and how draining continues after consumer drop.
    - SEAM-4 owns how completion is gated on the resulting finality signal.
- **Dependencies**:
  - None for documentation, but must remain consistent with `BH-C04` (SEAM-3) and DR-0012.
- **Verification**:
  - Review-time: semantics read clearly without reading other modules.
  - Compile-time: any doc examples (if included) compile, but keep them minimal.

## Atomic Tasks

#### S1.T1 — Add a module-level “BH-C05 semantics” note in `run_handle_gate.rs`

- **Outcome**: `crates/agent_api/src/run_handle_gate.rs` has a short, precise description of the gating semantics and the rationale (“prevent early completion relative to stream finality”).
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/run_handle_gate.rs` (doc comment / module note)
- **Implementation notes**:
  - Define “finality” in terms of the universal `mpsc::Receiver<AgentWrapperEvent>` reaching `None` (sender dropped).
  - Define “consumer drop” in terms of the `events` stream being dropped (and how that signals finality for gating purposes).
  - Call out the invariant that the harness must continue draining backend streams even after consumer drop (SEAM-3), even though those events are no longer observable.
- **Acceptance criteria**:
  - The doc is explicit enough that S3 tests can reference it as the source of truth for expectations.
- **Test notes**: none (tests are Slice S3).
- **Risk/rollback notes**: documentation-only; if the semantics are wrong, fix the doc first before changing implementation.

Checklist:
- Implement: `BH-C05` semantics doc block in `run_handle_gate.rs`.
- Validate: `make clippy` (doc/comment warnings if any).
- Cleanup: keep wording consistent with ADR-0013 and the seam briefs.

#### S1.T2 — Name the seam boundary: what SEAM-4 guarantees vs what SEAM-3 guarantees

- **Outcome**: A short “responsibility split” note that prevents hidden coupling between the pump (SEAM-3) and gate (SEAM-4).
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/run_handle_gate.rs` (or a short note in `crates/agent_api/src/backend_harness.rs` once it exists)
- **Implementation notes**:
  - SEAM-3 guarantee: sender finality (sender drop) happens only after the backend event stream ends, and draining continues after consumer drop.
  - SEAM-4 guarantee: completion cannot be observed before finality signal, except via consumer drop.
- **Acceptance criteria**:
  - The note is discoverable and referenced by S2/S3 implementation tasks.
- **Test notes**: none.
- **Risk/rollback notes**: prevents future “fixes” that accidentally split logic across modules.

Checklist:
- Implement: responsibility split note.
- Validate: no duplication or contradiction with existing comments.
- Cleanup: keep the note short; avoid re-stating full ADR text.
