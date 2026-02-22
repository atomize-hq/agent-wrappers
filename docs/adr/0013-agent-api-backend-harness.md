# ADR-0013 — `agent_api` backend harness for fast CLI onboarding
#
# Note: Run `make adr-fix ADR=docs/adr/0013-agent-api-backend-harness.md` after editing to update
# the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft
- Date (UTC): 2026-02-22
- Owner(s): spensermcconnell

## Scope
- Universal backend adapters in:
  - `crates/agent_api/src/backends/`
- Internal-only helper modules in:
  - `crates/agent_api/src/`

## Related Docs
- Universal Agent API baseline:
  - `docs/adr/0009-universal-agent-api.md`
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
  - `docs/specs/universal-agent-api/event-envelope-schema-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/extensions-spec.md`
- Current built-in backend adapters (implementation context):
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/claude_code.rs`
- Onboarding charter (process context):
  - `docs/project_management/next/cli-agent-onboarding-charter.md`

## Executive Summary (Operator)

ADR_BODY_SHA256: f50df499202efeb7d2f1a0a740fa5bfe5c1784c93b33875bc3ac56e84ddbc1ee

### Changes (operator-facing)

- Introduce a small internal “backend harness” inside `crates/agent_api`
  - Existing: each backend adapter re-implements the same glue concerns (request/extensions
    validation, env merge precedence, timeout wrapping, bounds enforcement, best-effort forwarding
    with drain-on-drop behavior, and DR-0012 completion gating integration).
  - New: centralize those glue concerns in a reusable internal module so onboarding new CLI agents
    is mostly “spawn + parse + map”.
  - Why: reduce duplication, make cross-backend behavior consistent by construction, and decrease
    time-to-onboard for many CLI wrappers.
- No change to the universal API surface or capability ids (implementation-only refactor).

## Problem / Context

The Universal Agent API is explicitly designed to support onboarding many different CLI agents.
The intended layering is:

- `crates/<agent>`: agent-specific wrapper library (spawn semantics, stream parsing, typed events)
- `crates/agent_api`: universal facade (capabilities, extensions validation, bounds, event envelope)

In practice, `crates/agent_api/src/backends/*.rs` tends to accumulate repeated “glue” logic across
backends. This has two consequences:

- onboarding a new CLI requires copying and re-auditing glue logic, and
- behavior can drift subtly between backends (especially around validation and safety invariants).

## Goals

- Make onboarding a new CLI agent backend mostly mechanical:
  - implement wrapper crate stream parsing (typed events + completion), then
  - implement a thin `agent_api` backend adapter with a mapping function.
- Centralize universal invariants and “glue” behavior in one place:
  - fail-closed extension validation
  - deterministic env merge precedence (backend defaults overridden by per-run request)
  - timeout wrapping
  - bounds enforcement for events and completion payloads
  - streaming forward behavior that drains backend streams even if consumer drops the universal
    events stream (avoid deadlocks/cancellation)
  - DR-0012 completion gating wiring (completion must not resolve until stream finality is observed
    or dropped by the consumer)

## Non-Goals

- Removing per-backend adapter modules entirely (each backend still needs a spawn + mapping layer).
- Merging agent wrapper crates into `agent_api` (one crate per agent CLI remains the model).
- Changing or expanding universal capability ids (this ADR is implementation-only).
- Creating a new type system for capabilities or extensions (open-set strings remain).

## User Contract (Authoritative)

This ADR MUST NOT change the public Rust API of `agent_api` or the normative universal spec set.
In particular:

- Capability advertising and extension keys remain as specified in:
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/extensions-spec.md`
- Run finality semantics remain DR-0012 / `run-protocol-spec.md`.
- Safety bounds and redaction behavior remain as specified in `event-envelope-schema-spec.md`.

## Proposed Architecture Shape

### Backend harness (internal module)

Add an internal module (name TBD; e.g. `agent_api::backend_harness`) which implements the common
run loop and enforces universal invariants.

Each backend adapter provides only:

- backend identity (`AgentWrapperKind`)
- capabilities list (including supported `extensions` keys)
- request validation that is backend-specific (supported keys + per-key JSON schema checks)
- spawn logic (into that backend’s wrapper crate)
- mapping from wrapper typed events into universal `AgentWrapperEvent` + completion extraction

The harness provides:

- a canonical validation flow for “unknown extension key => fail closed”
- canonical env merge semantics (backend config env, then per-request env overrides)
- canonical timeout wrapper behavior
- canonical “forward while receiver alive, but continue draining on receiver drop”
- canonical bounds enforcement integration points
- canonical DR-0012 gating integration (via existing gated handle builder)

### File/module boundaries (informative)

- `crates/agent_api/src/backend_harness.rs` (new; internal)
- `crates/agent_api/src/backends/codex.rs` and `.../claude_code.rs` refactored to use the harness
  (no behavior changes intended; smaller files)

## Alternatives Considered

- Do nothing (copy/paste glue per backend)
  - Rejected: onboarding cost grows linearly and drift risk increases.
- Use a macro per backend
  - Rejected: macros tend to hide control flow and make safety invariants harder to audit.
- Push all invariants into each wrapper crate
  - Rejected: those crates are agent-specific; universal invariants belong in the universal layer.

## Security / Safety Posture

This ADR is a net safety improvement because it reduces the number of places where:

- extension validation can become permissive by accident,
- raw backend line content can leak into errors/messages, and
- bounds enforcement and stream draining semantics can drift.

## Validation Plan (Authoritative)

- Refactor should be “no behavior change” at the spec level:
  - Run the existing integration tests for Codex and Claude backends.
  - Add harness unit tests that exercise:
    - env merge precedence
    - fail-closed unknown extension key behavior (per backend allowlist)
    - “receiver drop does not cancel backend draining” behavior
    - DR-0012 completion gating (completion pending until stream finality is observed/dropped)
- Repo checks:
  - `make fmt-check`
  - `make clippy`
  - `make test`

## Rollout / Backwards Compatibility

- Internal refactor only; no external rollout steps required.
- Backend adapter behavior must remain compatible with the existing universal contract.

## Decision Summary

Centralize universal backend “glue” behavior in a small internal harness module within
`crates/agent_api` to make onboarding new CLI agent backends faster and to reduce cross-backend
behavior drift, while keeping one crate per agent CLI and keeping per-backend adapters as thin
spawn+map layers.
