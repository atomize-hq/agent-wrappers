# Decision register — Explicit cancellation (`agent_api`)

This document records non-trivial A/B decisions required by ADR-0014.

## DR-CA-0001 — Public API shape for explicit cancellation

- Option A: add `AgentWrapperGateway::run_control(...) -> AgentWrapperRunControl` returning a
  cancellation handle alongside `AgentWrapperRunHandle`. (Selected)
  - Pros: additive; does not change existing `run(...)` signature; does not require making the run
    handle opaque or changing its field visibility.
  - Cons: introduces a second “run entrypoint” and type.
- Option B: add `AgentWrapperRunHandle::cancel()` directly.
  - Pros: single entrypoint; ergonomic.
  - Cons: difficult to implement without changing the public run handle struct shape (breaking) or
    making it opaque (also breaking).

Selection: **A**

## DR-CA-0002 — Completion error shape on cancellation

- Option A: represent cancellation as `AgentWrapperError::Backend { message: "cancelled" }`.
  - Pros: no breaking change to the public `AgentWrapperError` enum.
  - Cons: stringly-typed; cancellation is not structurally distinguishable.
- Option B: add `AgentWrapperError::Cancelled`.
  - Pros: structurally explicit; downstream can match reliably.
  - Cons: breaking change to a public enum (requires a compatibility policy decision).

Selection: **A** (v1)

Pinned message:
- `"cancelled"` is defined canonically in:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`, and
  - `docs/specs/universal-agent-api/run-protocol-spec.md`.

## DR-CA-0003 — Capability gating for explicit cancellation

- Option A: gate `run_control(...)` behind a new core capability id (`agent_api.control.cancel.v1`)
  and fail-closed when absent. (Selected)
  - Pros: consistent with existing capability gating posture; makes support explicit per backend.
  - Cons: requires adding a new core capability id.
- Option B: always provide `run_control(...)` but allow cancellation to be a no-op for unsupported
  backends.
  - Pros: simpler consumer code (no capability check).
  - Cons: cancellation becomes unreliable and ambiguous; weakens fail-closed posture.

Selection: **A**
