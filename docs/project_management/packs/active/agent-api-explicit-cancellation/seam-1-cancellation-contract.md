# SEAM-1 — Cancellation contract (CA-C01)

This seam pins the public API surface and exact cancellation semantics.

## Public API surface (v1, normative for this pack)

Add a new gateway method and new public types:

```rust
use std::future::Future;
use std::pin::Pin;

impl AgentWrapperGateway {
    pub fn run_control(
        &self,
        agent_kind: &AgentWrapperKind,
        request: AgentWrapperRunRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<AgentWrapperRunControl, AgentWrapperError>>
                + Send
                + '_,
        >,
    >;
}

pub struct AgentWrapperRunControl {
    pub handle: AgentWrapperRunHandle,
    pub cancel: AgentWrapperCancelHandle,
}

#[derive(Clone)]
pub struct AgentWrapperCancelHandle {
    // private
}

impl AgentWrapperCancelHandle {
    pub fn cancel(&self);
}
```

The existing `AgentWrapperGateway::run(...) -> AgentWrapperRunHandle` remains unchanged.

### Gateway error behavior (exact)

- Unknown backend:
  - `AgentWrapperGateway::run_control(...)` MUST return:
    - `Err(AgentWrapperError::UnknownBackend { agent_kind })`
    - where `agent_kind == <requested AgentWrapperKind>.as_str().to_string()`.
- Missing capability:
  - If the resolved backend does not advertise `agent_api.control.cancel.v1`, `run_control(...)` MUST return:
    - `Err(AgentWrapperError::UnsupportedCapability { agent_kind, capability })`
    - where:
      - `agent_kind == <requested AgentWrapperKind>.as_str().to_string()`, and
      - `capability == "agent_api.control.cancel.v1"`.

### Capability gating (exact)

- A backend supports explicit cancellation if and only if it advertises capability id:
  - `agent_api.control.cancel.v1`
- If the backend does not advertise `agent_api.control.cancel.v1`, `run_control(...)` MUST fail-closed
  with the full `UnsupportedCapability` shape (including `agent_kind`) as specified above.

## Cancellation semantics (exact)

- Calling `cancel()` MUST be idempotent.
- `cancel()` MUST be best-effort:
  - it requests that the underlying backend process terminate, and
  - it requests that harness driver tasks stop producing additional work.
- Completion outcome and precedence (pinned):
  - If cancellation is requested before `completion` resolves (i.e., before it would resolve as `Ok(...)` or `Err(...)`),
    `completion` MUST resolve to:
    - `Err(AgentWrapperError::Backend { message })` where `message == "cancelled"`.
  - If `completion` resolves before cancellation is requested, `cancel()` is a no-op and MUST NOT
    change the already-resolved completion value.
  - Tie-breaking (concurrent readiness): cancellation wins (the pinned `"cancelled"` error).

## Relationship to drop semantics

- Drop-based cancellation semantics remain as specified by `run-protocol-spec.md` (“best-effort”).
- Consumers requiring deterministic cancellation MUST use the explicit cancellation handle.
