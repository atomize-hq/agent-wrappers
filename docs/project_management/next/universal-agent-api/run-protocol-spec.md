# Run Protocol Spec — Universal Agent API

Status: Draft  
Date (UTC): 2026-02-16

This spec defines the lifecycle semantics for `agent_api` runs, event ordering, and completion.

## Run lifecycle

1. Caller constructs `AgentRunRequest` and an `AgentKind`.
2. Caller invokes `AgentGateway::run(&agent_kind, request)`.
3. `AgentGateway` resolves an `AgentBackend` for the `AgentKind`, otherwise returns `AgentError::UnknownBackend`.
4. Backend validates required capabilities for the requested operation.
5. Backend starts a run and returns an `AgentRunHandle`:
   - an event stream
   - a completion result future/value

## Streaming vs buffered events (DR-0001)

- Live streaming is not guaranteed across all agents.
- Backends MUST advertise whether they support live streaming via capabilities.
- Capability meaning (normative):
  - If a backend includes `agent_api.events.live`, the backend MUST be able to emit at least one
    `AgentEvent` before the underlying process exits for non-trivial runs (i.e., events are not
    purely post-hoc).
  - If a backend does not include `agent_api.events.live`, the backend MAY buffer and emit events
    only after completion.
- If a backend does not support live streaming:
  - it may buffer events and emit them after completion
  - it must still preserve event ordering relative to the buffered production

## Ordering guarantees

- Within a single `AgentRunHandle`, events are emitted in the order produced by the backend mapping.
- No cross-run ordering is implied.

## Cancellation semantics (minimum)

- Cancellation is best-effort:
  - If the caller drops the run handle / cancels the completion future, the backend should attempt to terminate the underlying process.
  - If termination fails, the backend must surface an error in the completion result.

## Capability validation timing

- Unsupported operations must fail-closed:
  - validate capabilities before spawning work where possible
  - if an operation becomes unsupported mid-run (backend error), complete with error and emit an `Error` event if feasible

## Required completion semantics (v1, normative)

- `AgentRunHandle.completion` MUST resolve exactly once.
- On success, `completion` MUST contain the underlying process `ExitStatus`.
- `AgentCompletion.final_text`:
  - MAY be populated when the backend can deterministically extract a “final” text response.
  - MUST be `None` if the backend cannot do so safely or deterministically.
