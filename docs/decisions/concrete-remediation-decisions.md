# Concrete remediation decisions

Date (UTC): 2026-02-24

Scope: Documentation-only remediation for explicit cancellation (`agent_api`), based on:
- `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/concrete-audit.report.json`

This document records concrete decisions introduced where the audit required an explicit choice and
no single authoritative source fully pinned the behavior.

## CRD-0001 — Explicit cancellation is not an exception to completion gating

**Decision**

Explicit cancellation MUST still respect DR-0012 completion gating:
- `completion` MUST NOT resolve until the underlying backend process has exited, and
- unless the consumer drops `events` (opt-out), the consumer-visible `events` stream is final (`None`).

**Context**

CA-0001 required specifying whether explicit cancellation is an exception to the completion gating
rule, and how cancellation interacts with the “no late events after completion” guarantee.

**Chosen spec**

Pinned in `docs/specs/universal-agent-api/run-protocol-spec.md` under:
- “Relationship between `completion` and the event stream (DR-0012 / v1, normative)”
- “Explicit cancellation semantics (v1, normative)” → “Completion gating”

**Rationale**

- Keeps `completion` as the reliable “the run is fully done (process exit reached)” signal, which
  lets tests treat `completion` resolution as the termination observation point.
- Preserves the “no late events after completion” invariant for consumers that keep `events` alive.

**Implications**

- Implementations MUST request best-effort termination on `cancel()` and resolve completion only
  after process exit.
- Consumers may observe stream finality before completion in cancellation paths.

## CRD-0002 — Consumer-visible event stream closes on cancellation and buffered events are dropped

**Decision**

After cancellation is requested, the backend MUST:
- stop forwarding additional events to the consumer-visible `events` stream, and
- close the consumer-visible `events` stream (consumer can observe `None`).

If the backend buffers events for post-hoc emission (non-live), any buffered events not yet emitted
MUST be dropped (MUST NOT be flushed to the consumer after cancellation).

**Context**

CA-0001 required pinning consumer-visible event-stream behavior and buffered-event handling after
`cancel()`.

**Chosen spec**

Pinned in `docs/specs/universal-agent-api/run-protocol-spec.md` under:
- “Explicit cancellation semantics (v1, normative)” → “Consumer-visible event stream behavior after `cancel()`”

**Rationale**

- Aligns with the SEAM-2 driver plan (“stop forwarding” + “close stream”) and provides deterministic
  behavior for orchestrators.
- Avoids ambiguous “late events after completion” interactions in cancellation flows.

**Implications**

- Implementations MUST separate internal draining (to avoid deadlocks) from consumer-visible
  forwarding.
- Consumers must not rely on receiving post-hoc buffered events after requesting cancellation.

## CRD-0003 — Cancellation outcome precedence and tie-breaking

**Decision**

If cancellation is requested before `completion` resolves, the completion outcome MUST be the pinned
cancellation error:
- `Err(AgentWrapperError::Backend { message: "cancelled" })`

This MUST override any backend `Ok(...)` or `Err(...)` completion that would otherwise resolve after
cancellation is requested.

If `completion` resolves first, cancellation is a no-op and MUST NOT change the already-resolved
value.

If cancellation and completion become ready concurrently, cancellation wins.

**Context**

CA-0002 required precedence rules for cancellation vs backend completion outcomes and explicit
tie-breaking for simultaneous readiness.

**Chosen spec**

Pinned in:
- `docs/specs/universal-agent-api/run-protocol-spec.md` → “Explicit cancellation semantics” → “Completion outcome and precedence (pinned)”
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` → “Completion outcome and precedence (pinned)”

**Rationale**

- Ensures deterministic cancellation behavior for orchestrators and tests.
- Avoids flake from race ambiguity and avoids exposing backend-specific kill/exit error shapes.

**Implications**

- Implementations should use cancellation-biased race resolution when cancellation and backend
  completion are simultaneously ready.

## CRD-0004 — SEAM-4 pinned timeouts and parameters

**Decision**

SEAM-4 tests use the following pinned parameters in v1:

- `FIRST_EVENT_TIMEOUT = 1s`
- `CANCEL_TERMINATION_TIMEOUT = 3s`
- `DROP_COMPLETION_TIMEOUT = 3s`
- `MANY_EVENTS_N = 200`

No platform-specific adjustment in v1 (same values on all supported platforms).

**Context**

CA-0007 required numeric timeout values, explicit pass/fail termination criteria, and pinned numeric
parameters for backpressure/drain regression tests.

**Chosen spec**

Pinned in:
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md`

**Rationale**

- Keeps CI-friendly “seconds, not minutes” budgets while matching existing repo patterns (other
  `agent_api` integration tests commonly use ~1–3 second `tokio::time::timeout` windows).

**Implications**

- Cancellation/termination implementations must satisfy these timeouts in CI on supported platforms.
