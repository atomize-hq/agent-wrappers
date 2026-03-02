# SEAM-3 — Streaming pump + drain-on-drop semantics

- **Name**: Shared stream forwarding and draining orchestration
- **Type**: risk
- **Goal / user value**: Make “live events + safe completion” behavior consistent across backends, including the critical invariant: if a consumer drops the universal events stream, the backend stream is still drained (to avoid deadlocks/cancellation).
- **Scope**
  - In:
    - A shared orchestration loop that:
      - forwards mapped/bounded events while the receiver is alive, and
      - continues draining backend events after receiver drop without forwarding.
    - A shared internal driver split where:
      - the **pump/drainer** (this seam) owns the event sender and drains the backend stream to end, and
      - a separate **completion sender** (SEAM-4) awaits the backend completion future and publishes the completion outcome.
      This avoids double-driving the completion future and keeps completion production independent of draining.
    - Canonical bounded channel sizing guidance and behavior (at minimum: no unbounded buffering).
  - Out:
    - Backend-specific mapping logic (still backend-owned) beyond a hook.
    - Changing the universal “live” semantics or DR-0012 finality rules.
- **Primary interfaces (contracts)**
  - Inputs:
    - Typed backend event stream
    - Mapping function (typed event/error → one or more `AgentWrapperEvent`s)
    - Sender for `AgentWrapperEvent` (bounded channel)
  - Outputs:
    - A drained-to-end backend stream with a well-defined finality signal (dropping the `AgentWrapperEvent` sender only at stream end).
    - (SEAM-4 consumes the finality signal to gate completion observability per DR-0012.)
- **Key invariants / rules**:
  - MUST NOT cancel the backend process/stream just because the universal receiver is dropped.
  - MUST keep draining until the backend stream ends.
    - No early-stop (“give up”) escape hatch is permitted under `BH-C04`. If an escape hatch is needed in the future, introduce it as an explicit contract change with pinned tests.
  - MUST apply `crate::bounds` to every forwarded event.

## Bounded channel sizing (BH-C04) (pinned)

The harness-owned events channel created by the canonical handle builder (BH-C05) MUST be bounded.

Default capacity (v1; pinned):

- `DEFAULT_EVENT_CHANNEL_CAPACITY: usize = 32`

Configuration (v1):

- Not configurable in v1. Any change to this constant MUST be treated as a behavior change and must
  update/pin the harness pump backpressure regression tests.

Rationale (non-normative): this preserves current behavior in both built-in backends, which
currently use `mpsc::channel::<AgentWrapperEvent>(32)`.
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
  - Harness-level tests using a fake stream that:
    - forces receiver drop mid-stream and asserts the backend stream is still fully drained, and
    - asserts at least one event can be forwarded before stream finality (live behavior).
- **Risks / unknowns**
  - Risk: accidental semantic change (ordering, cancellation, or backpressure) when unifying two distinct implementations.
  - De-risk plan: start by extracting Codex’s existing helper into the harness verbatim, then adapt Claude to it, keeping tests at each step.
- **Rollout / safety**:
  - Treat as the highest-risk seam; require explicit tests and comparison against existing behavior.

## Downstream decomposition prompt

Decompose into slices that (1) extract a shared “drain + forward” pump primitive, (2) pin drop semantics (forward flag + finality signaling), and (3) add a deterministic fake-stream test that fails if draining stops early.
