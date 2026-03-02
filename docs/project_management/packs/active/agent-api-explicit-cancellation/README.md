# Execution Pack — Explicit cancellation (`agent_api`)

Source ADR: `docs/adr/0014-agent-api-explicit-cancellation.md`

This pack defines the concrete contracts, seams, and tests required to add an explicit cancellation
API to `agent_api` runs without undermining the existing drain-on-drop safety posture.

## Canonical contracts (source of truth)

- Universal Agent API contract (public Rust surface): `docs/specs/universal-agent-api/contract.md`
- Run protocol semantics (DR-0012 + explicit cancellation): `docs/specs/universal-agent-api/run-protocol-spec.md`
- Capability id definition (`agent_api.control.cancel.v1`): `docs/specs/universal-agent-api/capabilities-schema-spec.md`

## Rollout / current support

Explicit cancellation is **specified** (and the specs are **Approved**) in the canonical Universal
Agent API spec set, but support is capability-gated per backend.

As of **2026-02-24** (this pack’s start date):
- Built-in backends do not yet advertise `agent_api.control.cancel.v1` (so the capability does not
  appear in the generated capability matrix), and `crates/agent_api` does not yet expose the
  `run_control(...)` public entrypoint.
- This execution pack (plus ADR-0014) is the plan-of-record for landing:
  - the `run_control(...)` API surface in `agent_api`,
  - harness wiring that preserves drain-on-drop + completion gating, and
  - built-in backend adoption + termination hooks + verification coverage.

**Source of truth for per-backend support**:
- `docs/specs/universal-agent-api/capability-matrix.md` (generated). If a capability id is absent,
  it is not advertised by any built-in backend.
