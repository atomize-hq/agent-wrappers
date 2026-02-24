# Run Protocol Spec — Universal Agent API

Status: Approved  
Approved (UTC): 2026-02-21  
Date (UTC): 2026-02-16

This spec defines the lifecycle semantics for `agent_api` runs, event ordering, and completion.

This document is normative and uses RFC 2119 keywords (MUST/SHOULD/MUST NOT).

## Run lifecycle

1. Caller constructs `AgentWrapperRunRequest` and an `AgentWrapperKind`.
2. Caller invokes `AgentWrapperGateway::run(&agent_kind, request)`.
3. `AgentWrapperGateway` resolves an `AgentWrapperBackend` for the `AgentWrapperKind`, otherwise returns `AgentWrapperError::UnknownBackend`.
4. Backend validates required capabilities for the requested operation.
5. Backend starts a run and returns an `AgentWrapperRunHandle`:
   - an event stream
   - a completion result future/value

## Streaming vs buffered events (DR-0001)

- Live streaming is not guaranteed across all agents.
- Backends MUST advertise whether they support live streaming via capabilities.
- Capability meaning (normative):
  - If a backend includes `agent_api.events.live`, the backend MUST be able to emit at least one
    `AgentWrapperEvent` before the underlying process exits for non-trivial runs (i.e., events are not
    purely post-hoc).
  - If a backend does not include `agent_api.events.live`, the backend MAY buffer and emit events
    only post-hoc (after the underlying process exits).
- If a backend does not support live streaming:
  - it may buffer events and emit them post-hoc (after the underlying process exits)
  - it must still preserve event ordering relative to the buffered production

## Relationship between `completion` and the event stream (DR-0012 / v1, normative)

Definitions:

- **Stream finality**: the consumer polls `AgentWrapperRunHandle.events` and receives `None`.
- **Consumer opt-out**: the consumer drops `AgentWrapperRunHandle.events` without draining to `None`.

Rules (pinned):

- If the consumer does **not** opt out (i.e., it keeps `events` alive), `AgentWrapperRunHandle.completion`
  MUST NOT resolve until:
  1) the underlying backend process has exited, and
  2) the consumer-visible `events` stream has reached finality.

- If the consumer opts out by dropping `events`, `completion` MAY resolve after (1) without waiting for (2).

This ensures that if a consumer observes `completion` resolving while still holding `events`, the
consumer-visible stream is final (no late-arriving events).

Backends that buffer events (non-live):
- A backend that buffers events for post-hoc emission MUST ensure those buffered events are emitted
  to the consumer-visible stream before stream finality, except when explicit cancellation closes
  the stream early (see below).

## Ordering guarantees

- Within a single `AgentWrapperRunHandle`, events are emitted in the order produced by the backend mapping.
- No cross-run ordering is implied.

## Cancellation semantics (minimum)

- Cancellation is best-effort:
  - Dropping `AgentWrapperRunHandle` (or dropping its `events` stream) is a best-effort cancellation
    signal. Backends MAY attempt to terminate the underlying process, but are not required to do so.
  - Consumers requiring deterministic cancellation MUST use the explicit cancellation handle
    (`run_control(...)`) when available.

## Explicit cancellation semantics (v1, normative)

When a caller uses `AgentWrapperGateway::run_control(...)`, the returned `AgentWrapperCancelHandle`
provides an explicit cancellation signal that is orthogonal to drop semantics.

- Capability gating:
  - Backends MUST advertise `agent_api.control.cancel.v1` if and only if they support explicit
    cancellation.
  - If a backend does not advertise `agent_api.control.cancel.v1`, `run_control(...)` MUST fail-closed
    with:
    - `AgentWrapperError::UnsupportedCapability { agent_kind, capability }`
    - where:
      - `capability == "agent_api.control.cancel.v1"`, and
      - for the gateway entrypoint, `agent_kind == <requested AgentWrapperKind>.as_str().to_string()`.

- Cancel handle lifetime (orthogonal):
  - `AgentWrapperCancelHandle::cancel()` MUST still function even if the caller drops:
    - `AgentWrapperRunControl.handle.events`, and/or
    - `AgentWrapperRunControl.handle` entirely.
  - If the run has already reached a terminal completion state, `cancel()` is a no-op.

- Consumer-visible event stream behavior after `cancel()`:
  - Once cancellation is requested, the backend MUST:
    - stop forwarding any additional `AgentWrapperEvent` items to the consumer, and
    - close the consumer-visible `events` stream (so the consumer can observe stream finality).
  - Events already successfully forwarded before the backend observes cancellation MAY still be received.
  - Buffered (non-live) events that have not yet been emitted MUST be dropped (MUST NOT be emitted after cancellation).

- Calling `AgentWrapperCancelHandle::cancel()` MUST be idempotent.
- Backends MUST attempt best-effort termination of the underlying backend process when `cancel()`
  is invoked.

- Completion outcome and precedence (pinned):
  - If cancellation is requested before `completion` resolves (i.e., before it would resolve as `Ok(...)` or `Err(...)`),
    `completion` MUST resolve to:
    - `Err(AgentWrapperError::Backend { message })` where `message == "cancelled"`.
    This MUST override any backend error completion that would otherwise occur after cancellation is requested.
  - If `completion` resolves before cancellation is requested, `cancel()` is a no-op and MUST NOT
    change the resolved completion value.
  - If cancellation and backend completion become ready concurrently, cancellation wins (the pinned `"cancelled"` error).

- Completion gating:
  - Explicit cancellation is NOT an exception to the completion/event-stream finality rules above:
    - if the consumer keeps `events` alive, the `"cancelled"` completion MUST NOT resolve until the
      consumer-visible `events` stream is closed (final) and the underlying backend process has exited;
    - if the consumer drops `events`, the `"cancelled"` completion MUST NOT resolve until the underlying
      backend process has exited.

## Capability validation timing

- Unsupported operations must fail-closed:
  - validate capabilities before spawning work where possible
  - if an operation becomes unsupported mid-run (backend error), complete with error and emit an `Error` event if feasible

Extension validation (v1, normative):
- Backends MUST validate `AgentWrapperRunRequest.extensions` keys and values before spawning any backend process.

## Required completion semantics (v1, normative)

- `AgentWrapperRunHandle.completion` MUST resolve exactly once.
- On success, `completion` MUST contain the underlying process `ExitStatus`.
- `AgentWrapperCompletion.final_text`:
  - MAY be populated when the backend can deterministically extract a “final” text response.
  - MUST be `None` if the backend cannot do so safely or deterministically.
