# ADR-0012 — Universal Agent API extensions registry + CLI agent onboarding charter
#
# Note: Run `make adr-fix ADR=docs/adr/0012-universal-agent-api-extensions-registry-and-cli-agent-onboarding-charter.md`
# after editing to update the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft
- Date (UTC): 2026-02-20
- Owner(s): spensermcconnell

## Scope
- Universal spec set:
  - `docs/project_management/next/universal-agent-api/`
- Onboarding charter doc:
  - `docs/project_management/next/cli-agent-onboarding-charter.md`

## Related Docs
- Universal Agent API baselines:
  - `docs/adr/0009-universal-agent-api.md`
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`
- New spec introduced by this ADR:
  - `docs/project_management/next/universal-agent-api/extensions-spec.md`
- Onboarding charter introduced by this ADR:
  - `docs/project_management/next/cli-agent-onboarding-charter.md`
- Current exec-policy usage in a feature pack (context; not authoritative for core keys):
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`

## Executive Summary (Operator)

ADR_BODY_SHA256: fc030dc1a8ef3f108a22096b616b11d8c411744da34add4f775f22893ea3c417

### Changes (operator-facing)

- Add a canonical registry for core `agent_api.*` extension keys
  - Existing: core extension semantics can be introduced ad-hoc inside feature packs, risking drift
    and contradictory defaults across backends over time.
  - New: core extension keys (schema + defaults + absence semantics + ownership rules) live in a
    single authoritative spec (`extensions-spec.md`), and feature packs defer to it.
  - Why: prevents “implied” extension semantics and makes onboarding new CLIs mechanical.
  - Links:
    - `docs/project_management/next/universal-agent-api/extensions-spec.md`

- Add a repo-local charter for onboarding new CLI agent crates + universal backend adapters
  - Existing: onboarding rules are implicit (spread across prior packs/ADRs and local convention).
  - New: a single charter documents the canonical layers, capability/extension rules, mapping
    rubric, test evidence expectations, and CI expectations.
  - Why: enables adding many more CLI agents quickly without re-deciding fundamentals each time.
  - Links:
    - `docs/project_management/next/cli-agent-onboarding-charter.md`

## Problem / Context

The universal agent API is explicitly designed to support many CLI agents over time. The primary
drift risk is that new behaviors (especially `extensions`) get introduced inside feature-local
packs without a single canonical “core extensions” registry.

When this happens:
- different packs may define different defaults/absence semantics for the same core key,
- backends can diverge in validation and fail-closed behavior, and
- future onboarding work slows down because each new agent re-discovers and re-documents rules.

## Goals

- Define a canonical, universal registry for core extension keys (`agent_api.*`).
- Define deterministic ownership rules for extension keys:
  - core keys are owned by the universal registry
  - backend keys are owned by backend docs
- Provide a single “onboarding charter” describing the canonical rules and the intended orthogonal
  architecture layers for future CLI agents.

## Non-Goals

- Introducing new planning-doc enforcement or CI gating for planning artifacts.
- Implementing new execution features beyond documenting and pinning the rules.
- Forcing identical tool payload schemas across all agents.

## User Contract (Authoritative)

### Universal API (`agent_api`)

- `AgentWrapperRunRequest.extensions` remains an open map of keys to JSON values.
- Canonical semantics for core keys under `agent_api.*` are defined in:
  - `docs/project_management/next/universal-agent-api/extensions-spec.md`

Initial core key pinned by this ADR:
- `agent_api.exec.non_interactive` (boolean; defaults to `true` when absent).

### Ownership rules (contract)

- Core keys (`agent_api.*`) MUST be defined in `extensions-spec.md`.
- Backend keys (`backend.<agent_kind>.*`) MUST be defined in the backend’s authoritative
  contract/spec docs and MUST NOT be defined in `extensions-spec.md`.

### Platform guarantees

No new platform divergences are introduced by this ADR (documentation-only change).

## Architecture Shape

- Add a new universal spec:
  - `docs/project_management/next/universal-agent-api/extensions-spec.md`
    - core extension key registry (schema + defaults + validation)
    - ownership rules for backend keys
- Add a new charter doc:
  - `docs/project_management/next/cli-agent-onboarding-charter.md`
    - wrapper crate rules
    - universal backend adapter rules
    - mapping rubric (“capability buckets”)
    - test evidence expectations and CI expectations

## Dependencies

- ADR-0009 defines the universal API and capability gating model:
  - `docs/adr/0009-universal-agent-api.md`

## Security / Safety Posture

- The registry and charter explicitly reinforce:
  - fail-closed validation for unknown extension keys,
  - non-interactive defaults as a safety requirement for automation, and
  - redaction/no-raw-line rules for v1 universal events and errors.

## Validation Plan (Authoritative)

- `make adr-check ADR=docs/adr/0012-universal-agent-api-extensions-registry-and-cli-agent-onboarding-charter.md`
- Cross-document scan (manual): ensure no other doc defines conflicting semantics for
  `agent_api.exec.non_interactive`.

## Rollout / Backwards Compatibility

- Greenfield posture: documentation-only. No compatibility policy is required.

## Decision Summary

- No decision register is required: this ADR introduces a single, unambiguous documentation
  ownership policy and a deterministic core extensions registry.

