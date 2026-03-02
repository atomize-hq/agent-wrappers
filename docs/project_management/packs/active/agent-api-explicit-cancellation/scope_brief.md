# Scope brief — Explicit cancellation (`agent_api`)

## Goal

Add an explicit cancellation control plane primitive for `agent_api` runs so orchestrators can
reliably request backend termination independent of consumer-side timeouts and independent of drop
semantics.

## Non-goals

- Introduce a universal/global default timeout in the Universal Agent API spec.
- Make cancellation synchronous or guarantee immediate termination on all platforms.
- Expose backend-specific process handles in the public API.
- Expand the universal event schema to include raw backend output or unbounded cancellation payloads.

## Required invariants (must not regress)

- **Run finality**: completion gating and “no late events after completion” semantics remain per the
  Universal Agent API run protocol.
- **Safety posture**: bounded/redacted events and completion payload bounds remain unchanged.
- **Drain-on-drop**: dropping the universal events receiver must not deadlock or stall backend
  draining; explicit cancellation must not reintroduce cancellation-induced deadlocks.

## Primary contract surfaces

- Public Rust API surface in `agent_api` (`docs/specs/universal-agent-api/contract.md`):
  - new types for cancellation control
  - new gateway method that returns a cancellation handle alongside the existing run handle
- Run protocol semantics (`docs/specs/universal-agent-api/run-protocol-spec.md`):
  - explicit cancellation semantics, including completion error shape
  - relationship to drop semantics (“best-effort” remains but is not relied upon)

