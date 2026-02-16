# ADR 0009: Universal Agent API (Core + Feature-Gated Backends + Capabilities)

Date: 2026-02-16  
Status: Proposed  
Owner(s): spensermcconnell

## Scope

- Feature directory: `docs/project_management/next/universal-agent-api/` (planning + triads)
- Sequencing spine: `docs/project_management/next/sequencing.json`
- Standards (format source): `/Users/spensermcconnell/__Active_Code/atomize-hq/substrate/docs/project_management/standards/ADR_STANDARD_AND_TEMPLATE.md`
- Repo docs conventions: `docs/STYLE.md`

## Related Docs

- Existing ingestion contract ADR (reused, not replaced): `docs/adr/0007-wrapper-events-ingestion-contract.md`
- Existing workspace shape ADR (context): `docs/adr/0006-agent-wrappers-workspace.md`
- Feature plan/tasks:
  - `docs/project_management/next/universal-agent-api/plan.md`
  - `docs/project_management/next/universal-agent-api/tasks.json`
  - `docs/project_management/next/universal-agent-api/session_log.md`
  - `docs/project_management/next/universal-agent-api/decision_register.md`
- Spec manifest (derived, authoritative for spec set): `docs/project_management/next/universal-agent-api/spec_manifest.md`
- Impact map (derived, authoritative for touch set + conflicts): `docs/project_management/next/universal-agent-api/impact_map.md`
- CI checkpoint plan (derived, authoritative for bounded multi-OS gates): `docs/project_management/next/universal-agent-api/ci_checkpoint_plan.md`
- Contract/spec docs (authoritative):
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/platform-parity-spec.md`
- Manual/smoke validation:
  - `docs/project_management/next/universal-agent-api/manual_testing_playbook.md`
  - `docs/project_management/next/universal-agent-api/smoke/`
- Wrapper crates (inputs):
  - `crates/codex/`
  - `crates/claude_code/`
  - `crates/wrapper_events/`

## Executive Summary (Operator)

ADR_BODY_SHA256: 5eacaa112086896ecfc73b0ae27e93d2ee5dcc594465eacb4a791911b677f679

### Changes (operator-facing)

- Add a new “universal” Rust API crate for multi-agent usage
  - Existing: Consumers integrate per-agent (`codex`, `claude_code`) with different request builders, different streaming event types, and bespoke capability probing.
  - New: Consumers depend on a single crate (`agent_api`) with a unified run/event contract and explicit capability discovery; backend choice becomes a parameter (agent kind) plus optional capability-specific calls.
  - Why: Enables adding many CLI agents without reshaping consumer code or forcing a “least common denominator” API.
  - Links:
    - `docs/adr/0009-universal-agent-api.md#user-contract-authoritative`
    - `docs/adr/0009-universal-agent-api.md#architecture-shape`

## Problem / Context

This repo is expanding from a single wrapper (Codex) to multiple wrappers (Claude Code and future
agents). Today, each wrapper exposes its own builder/request types and its own streaming/event
shapes. Downstream consumers that want “pick an agent and run a task” must:

- write per-agent glue code (request mapping + env/defaults + parsing)
- branch their orchestration logic on agent type
- add one-off capability detection and custom flows per agent

We need a stable, orthogonal “universal API” layer that allows a consumer to:

1) select an agent backend by identity,  
2) run a standard task contract, and  
3) consume a unified event stream,  
while still allowing agent-specific capabilities without distorting the core API.

## Goals

- Provide a single Rust crate that exposes a unified async “run” contract across many agent CLIs.
- Make agent selection an input (not a type-level fork) for common operations.
- Make capability discovery explicit and queryable at runtime.
- Support extensions without breaking the core contract (agent-specific ops live behind capabilities).
- Preserve existing wrapper crates (`codex`, `claude_code`) as stable, independently usable libraries.

## Non-Goals

- Wrapping interactive/TUI modes for upstream CLIs (universal API targets headless flows only).
- Replacing `wrapper_events` with a new ingestion boundary (it remains the shared ingestion primitive).
- Defining a Substrate-style “AgentEvent envelope” or correlation model in this repo.
- Guaranteeing perfect semantic parity across agents (capabilities differ; the core API must reflect that).

## User Contract (Authoritative)

### Rust API surface

Introduce a new crate:

- `crates/agent_api` (published name: `agent_api`)

Contract:

- The crate exposes:
  - a stable agent identity type (`AgentKind`) that is an **open set** (string-backed; supports unknown/future agents)
  - a unified async execution surface (`AgentGateway` + `AgentBackend` + `AgentRunHandle`)
  - a unified event envelope (`AgentEvent`) with a small stable core and an extension payload
  - an explicit capability model (`AgentCapabilities`) used to gate optional operations
- The crate MUST NOT require consumers to depend directly on `codex`/`claude_code` unless they enable
  the corresponding feature flags.

### Backend selection

- A consumer selects a backend by `AgentKind` at runtime.
- The universal API does not force a compile-time enum expansion for new agents.

### Capability model

- `AgentCapabilities` declares:
  - which “core” operations are available (e.g., prompt/run with streaming events)
  - which optional extensions are available (named capabilities, string-backed)
- If an operation is invoked that is not supported, the call fails with a structured
  `UnsupportedCapability { agent_kind, capability }` error (not a panic, not a silent no-op).

### Event stream contract

The event stream is a unified, minimal contract:

- Each event includes:
  - `agent_kind` (string-backed identity)
  - `kind` (Text/ToolCall/ToolResult/Status/Error/Unknown)
  - `channel` (optional; best-effort)
  - `data` (optional `serde_json::Value` for agent-specific structured payload)
- The universal contract does not require identical tool payload schemas across agents.
- Raw/opaque payload is permitted only in `data` and must be safe-by-default (see Security).

### Defaults and environment isolation

- The universal API MUST NOT mutate the parent process environment.
- Per-run environment overrides are passed only to the spawned backend process.
- Timeouts are explicit:
  - if unset, backend defaults apply (wrapper crate defaults remain the source of truth)

## Architecture Shape

### Components

- `crates/agent_api`:
  - Core types/traits:
    - `AgentKind` (open set identity)
    - `AgentCapabilities`
    - `AgentRequest` / `AgentRunRequest` (core request with extension options)
    - `AgentEvent` (unified event envelope)
    - `AgentBackend` (trait)
    - `AgentGateway` (backend registry + routing)
  - Feature-gated backends (no default features):
    - `agent_api/codex` feature: backend implemented via `codex` crate
    - `agent_api/claude_code` feature: backend implemented via `claude_code` crate
  - Optional runtime support feature:
    - `agent_api/tokio` feature for tokio-backed streaming utilities (if needed)

### End-to-end flow

- Inputs:
  - `AgentKind`
  - `AgentRunRequest` (core request + optional extension options)
- Derived state:
  - backend resolution (`AgentGateway`)
  - capability validation (`AgentCapabilities`)
- Actions:
  - spawn backend wrapper client (`codex` or `claude_code`) and start the run
  - map backend-specific events into `AgentEvent`
- Outputs:
  - `AgentRunHandle`:
    - `events()` stream of `AgentEvent`
    - `wait()` completion result (exit status / final response summary as applicable)

## Sequencing / Dependencies

- Sequencing entry: `docs/project_management/next/sequencing.json` → add a new track
  sourced by this ADR under `docs/project_management/next/universal-agent-api/`.
- Dependencies:
  - Reuses `wrapper_events` patterns (feature-gated adapters, normalized event kinds).
  - Must not break existing public APIs of `codex` and `claude_code`.

## Security / Safety Posture

- Fail-closed rules:
  - Unsupported operations return `UnsupportedCapability` errors.
  - If backend resolution fails (unknown agent kind with no registered backend), return a structured
    `UnknownBackend` error.
- Secret handling:
  - The universal API does not retain raw backend output by default.
  - Any agent-specific payload in `AgentEvent.data` must be bounded and must not include raw lines
    unless the caller explicitly opts in via an extension option.
- Observability:
  - Events must carry `agent_kind` and an optional `run_id`/correlation id when available.

## Validation Plan (Authoritative)

### Tests

- Unit tests (in `crates/agent_api`):
  - backend registry routing and unknown-backend errors
  - capability gating behavior (supported vs unsupported)
  - event mapping invariants (kind/channel presence rules)
- Integration tests:
  - “sample/fixture” runs that do not require installed binaries (mirror the existing approach used
    by wrapper examples/fixtures where applicable).

### Manual validation

- Manual playbook will be created under `docs/project_management/next/universal-agent-api/` once the
  ADR is accepted and the triad feature scaffold exists.

## Rollout / Backwards Compatibility

- Policy: greenfield breaking is allowed for the new crate; existing crates must remain compatible.
- Compat work: none (no consumer migration is required to keep using existing wrappers directly).

## Decision Summary

- This ADR is intentionally self-contained: this repo’s existing ADR set does not use a separate
  Decision Register pattern. If the universal API work expands into multiple competing designs, a
  feature-local `decision_register.md` will be introduced under
  `docs/project_management/next/universal-agent-api/`.
