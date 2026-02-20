# Spec Manifest — Universal Agent API

Status: Draft  
Date (UTC): 2026-02-16  
Feature directory: `docs/project_management/next/universal-agent-api/`

## ADR Inputs

- ADR: `docs/adr/0009-universal-agent-api.md`

## Required planning artifacts (always)

- `docs/project_management/next/universal-agent-api/plan.md`
- `docs/project_management/next/universal-agent-api/tasks.json`
- `docs/project_management/next/universal-agent-api/session_log.md`
- `docs/project_management/next/universal-agent-api/ci_checkpoint_plan.md`

## Required cross-platform / decision-heavy artifacts

- `docs/project_management/next/universal-agent-api/decision_register.md`
- `docs/project_management/next/universal-agent-api/impact_map.md`
- `docs/project_management/next/universal-agent-api/manual_testing_playbook.md`
- `docs/project_management/next/universal-agent-api/quality_gate_report.md`
- Smoke scripts:
  - `docs/project_management/next/universal-agent-api/smoke/linux-smoke.sh`
  - `docs/project_management/next/universal-agent-api/smoke/macos-smoke.sh`
  - `docs/project_management/next/universal-agent-api/smoke/windows-smoke.ps1`

## Required spec documents (deterministic set)

### Slice specs (triads)

- `docs/project_management/next/universal-agent-api/C0-spec.md` — core `agent_api` crate surface + gateway + capability model + unified event envelope (no real backends).
- `docs/project_management/next/universal-agent-api/C1-spec.md` — Codex backend adapter (feature-gated) + event mapping + minimal “sample/fixture” validation.
- `docs/project_management/next/universal-agent-api/C2-spec.md` — Claude Code backend adapter (feature-gated) + event mapping + minimal “sample/fixture” validation.

### Contract spec (user-facing)

Trigger: ADR introduces a new user-facing library contract (public Rust API).

- `docs/project_management/next/universal-agent-api/contract.md` — authoritative contract for:
  - public Rust API surface (types, traits, feature flags)
  - error taxonomy for the universal API (including unsupported capability + unknown backend)
  - defaults (timeouts/env isolation requirements) and absence semantics

### Protocol spec (API semantics)

Trigger: ADR introduces a stable “run + streaming events + completion” execution protocol.

- `docs/project_management/next/universal-agent-api/run-protocol-spec.md` — authoritative semantics for:
  - request lifecycle (start, stream, completion)
  - ordering guarantees, cancellation semantics, backpressure expectations
  - capability gating behavior (when to validate; how to report)

### Schema spec (data model stability)

Trigger: ADR introduces a normalized event envelope and a capability model that must remain stable.

- `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md` — authoritative schema + constraints for `AgentWrapperEvent` (and any serialized representation if applicable).
- `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md` — authoritative schema + constraints for `AgentWrapperCapabilities` (open-set capability ids, stability rules, naming).
- `docs/project_management/next/universal-agent-api/extensions-spec.md` — authoritative registry + ownership rules for `AgentWrapperRunRequest.extensions` (core keys under `agent_api.*`).

### Platform parity spec

Trigger: ADR targets multi-platform spawning/streaming with feature-gated backends and must not “accidentally” become macOS-only.

- `docs/project_management/next/universal-agent-api/platform-parity-spec.md` — authoritative guarantees/divergences for Linux/macOS/Windows for:
  - process spawning expectations
  - streaming behavior (line endings, encoding assumptions)
  - feature availability and gating (what is allowed to differ)

## CI checkpoint plan

Trigger: cross-platform validation is required, and we want bounded CI gates rather than per-slice multi-OS runs.

- `docs/project_management/next/universal-agent-api/ci_checkpoint_plan.md` — authoritative checkpoint grouping + required CI gates + `tasks.json` wiring.

## Coverage matrix (surface → authoritative doc)

| Surface | Owner doc |
|---|---|
| Public Rust API (types/traits/builders) | `contract.md` |
| Cargo feature flags (`codex`, `claude_code`, any runtime features) | `contract.md` |
| Error taxonomy (`UnknownBackend`, `UnsupportedCapability`, mapping rules) | `contract.md` |
| Agent identity (`AgentWrapperKind` open-set rules) | `capabilities-schema-spec.md` |
| Capabilities: naming, stability, gating rules | `capabilities-schema-spec.md` + `run-protocol-spec.md` |
| Core extension key schemas + defaults (`agent_api.*`) | `extensions-spec.md` |
| Unified event envelope fields and invariants | `event-envelope-schema-spec.md` |
| What goes into event `data` vs redacted/safe defaults | `event-envelope-schema-spec.md` |
| Run lifecycle, ordering, cancellation, completion | `run-protocol-spec.md` |
| Backend adapter obligations (mapping + safety bounds) | `run-protocol-spec.md` |
| Platform guarantees/divergences | `platform-parity-spec.md` |
| Slice-specific deliverables/acceptance/out-of-scope | `C0-spec.md` / `C1-spec.md` / `C2-spec.md` |
| Checkpoint grouping + CI gate commands | `ci_checkpoint_plan.md` |
| Architectural decisions (A/B pinned) | `decision_register.md` |
| Manual/non-gating validation steps | `manual_testing_playbook.md` |

## Determinism checklist (per spec)

### `contract.md`

- Enumerates every public type/trait/function intended to be stable (or explicitly marks unstable).
- Defines feature flags and what symbols are available under each.
- Defines absence semantics (e.g., unset timeout, unset working dir, unknown agent kind).
- Defines error taxonomy (names + meaning + when emitted).

### `run-protocol-spec.md`

- Defines streaming ordering: what “in order” means, and what is allowed to interleave.
- Defines cancellation: how a run is terminated and what downstream observables must occur.
- Defines when capability validation happens (pre-run vs mid-run) and error behavior.
- Defines completion result semantics (what “success” means across agents).

### `extensions-spec.md`

- Enumerates core extension keys under `agent_api.*` and pins:
  - schema (type/allowed values)
  - defaults + absence semantics
  - validation and contradiction rules
  - ownership rules for backend-specific keys (`backend.<agent_kind>.*`)

### `event-envelope-schema-spec.md`

- Defines `AgentWrapperEvent.kind` mapping rules (including `Unknown` behavior).
- Defines `channel` semantics (optional, best-effort; allowed values; stability).
- Defines `data` bounds (size limits), redaction rules, and the v1 prohibition on raw backend line capture.
- Defines forward/backward policy if events are serialized/stored.

### `capabilities-schema-spec.md`

- Defines open-set naming rules for agent kinds and capability ids.
- Defines stability expectations (which ids are reserved/stable vs experimental).
- Defines “unsupported capability” semantics (what it means; how to detect).

### `platform-parity-spec.md`

- Defines per-platform guarantees for spawning and streaming.
- Defines allowed divergences and the exact validation evidence required.
