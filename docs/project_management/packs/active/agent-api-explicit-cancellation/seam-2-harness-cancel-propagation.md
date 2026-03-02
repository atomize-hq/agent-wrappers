# SEAM-2 — Harness cancellation propagation (CA-C02)

This seam pins how the backend harness observes cancellation without regressing drain-on-drop.

## Backend harness invariants referenced (pinned)

This pack uses **BH-C0x** shorthand for pre-existing backend-harness invariants that cancellation
MUST NOT regress.

### BH-C04 — Drain-on-drop posture

- If the consumer drops the universal `events` receiver/stream, the harness:
  - stops forwarding events, but
  - continues draining the typed backend event stream to completion (to avoid deadlocks /
    cancellation hazards).
- Canonical background: `docs/adr/0013-agent-api-backend-harness.md` (drain-on-drop posture).

### BH-C05 — Completion gating / consumer opt-out (DR-0012)

- `completion` timing is gated by the run protocol:
  - `completion` MUST NOT resolve until the underlying backend process has exited; and
  - if the consumer keeps `events` alive, `completion` MUST NOT resolve until the consumer-visible
    `events` stream is final (`None`).
  - if the consumer opts out by dropping `events`, `completion` MAY resolve after process exit
    without waiting for stream finality.
- Canonical: `docs/specs/universal-agent-api/run-protocol-spec.md` (DR-0012).

## Requirements

- The harness must continue draining backend event streams even if the consumer drops the universal
  receiver (BH-C04).
- Explicit cancellation must be orthogonal:
  - it must not depend on receiver drop, and
  - it must not depend on consumer-side timeout wrappers.
  - it must satisfy the cancel-handle lifetime guarantee in `docs/specs/universal-agent-api/run-protocol-spec.md`
    (“Cancel handle lifetime (orthogonal)”), i.e. `cancel()` must still function even if the caller
    drops `events` and/or drops the run handle.
- Cancellation MUST NOT violate completion gating (BH-C05 / DR-0012):
  - cancellation changes the completion *value*, not the completion *timing*.

## Driver model (pinned)

The harness run driver is split into:

- Pump/drainer task: forwards events while receiver is alive; always drains to stream end.
- Completion sender task: awaits backend completion; sends completion outcome independently.

Explicit cancellation introduces a third shared signal:

- A cancellation signal is observed by both tasks.
- On cancellation:
  - the pump/drainer stops forwarding but MUST still drain as required,
  - the backend process is requested to terminate (best-effort), and
  - the completion sender selects the pinned cancellation error if cancellation is requested before backend completion
    resolves (i.e., before it would resolve as `Ok(...)` or `Err(...)`); if cancellation and backend completion become
    ready concurrently, cancellation wins.
    This is completion *value* selection only and MUST still obey completion gating (BH-C05 / DR-0012): completion MUST
    NOT resolve before process exit, and MUST wait for consumer-visible stream finality unless the consumer opts out by
    dropping `events`.
