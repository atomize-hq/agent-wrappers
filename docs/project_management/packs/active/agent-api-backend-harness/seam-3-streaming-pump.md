# SEAM-3 — Streaming pump + drain-on-drop semantics

- **Name**: Shared stream forwarding and draining orchestration
- **Type**: risk
- **Goal / user value**: Make “live events + safe completion” behavior consistent across backends, including the critical invariant: if a consumer drops the universal events stream, the backend stream is still drained (to avoid deadlocks/cancellation).
- **Scope**
  - In:
    - A shared orchestration loop that:
      - forwards mapped/bounded events while the receiver is alive, and
      - continues draining backend events after receiver drop without forwarding.
    - A shared pattern for polling completion concurrently with draining events (backend-specific completion futures must not be canceled accidentally).
    - Canonical bounded channel sizing guidance and behavior (at minimum: no unbounded buffering).
  - Out:
    - Backend-specific mapping logic (still backend-owned) beyond a hook.
    - Changing the universal “live” semantics or DR-0012 finality rules.
- **Primary interfaces (contracts)**
  - Inputs:
    - Typed backend event stream
    - Completion future
    - Mapping function (typed event/error → one or more `AgentWrapperEvent`s)
    - Sender for `AgentWrapperEvent` (bounded channel)
  - Outputs:
    - A completion outcome that only resolves after the backend stream has ended (or after the consumer drop semantics are satisfied), per DR-0012 wiring expectations.
- **Key invariants / rules**:
  - MUST NOT cancel the backend process/stream just because the universal receiver is dropped.
  - MUST keep draining until backend stream ends (or until a defined “give up” condition that is explicitly justified; ADR-0013 implies full drain).
  - MUST apply `crate::bounds` to every forwarded event.
- **Dependencies**
  - Blocks:
    - `SEAM-5` — backend adoption should reuse this pump rather than having per-backend draining loops.
  - Blocked by:
    - `SEAM-1` — needs the harness contract shape (what is a “typed event stream” and “completion future”).
- **Touch surface**:
  - Existing exemplars to unify:
    - `crates/agent_api/src/backends/codex.rs` (`drain_events_while_polling_completion`)
    - `crates/agent_api/src/backends/claude_code.rs` (inline drain/forward loop)
  - Target: `crates/agent_api/src/backend_harness.rs` (shared pump implementation)
- **Verification**:
  - Harness-level tests using a fake stream + completion future that:
    - forces receiver drop mid-stream and asserts the backend stream is still fully drained, and
    - asserts at least one event can be forwarded before completion resolves (live behavior).
- **Risks / unknowns**
  - Risk: accidental semantic change (ordering, cancellation, or backpressure) when unifying two distinct implementations.
  - De-risk plan: start by extracting Codex’s existing helper into the harness verbatim, then adapt Claude to it, keeping tests at each step.
- **Rollout / safety**:
  - Treat as the highest-risk seam; require explicit tests and comparison against existing behavior.

## Downstream decomposition prompt

Decompose into slices that (1) extract a shared “drain while polling completion” primitive, (2) pin drop semantics (forward flag), and (3) add a deterministic fake-stream test that fails if draining stops early.

