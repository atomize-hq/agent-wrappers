# ADR-0019 — External sandbox execution policy (dangerous) (`agent_api.exec.external_sandbox.v1`)
#
# Note: Run `make adr-fix ADR=docs/adr/0019-universal-agent-api-external-sandbox-exec-policy.md`
# after editing to update the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft (implementation plan; normative semantics are pinned in the Universal Agent API specs)
- Date (UTC): 2026-03-04
- Owner(s): spensermcconnell

## Scope

Define a dangerous, explicit exec-policy knob for **externally sandboxed hosts**:

- Core extension key: `agent_api.exec.external_sandbox.v1` (boolean)

This work item corresponds to backlog id `uaa-0016` (`bucket=agent_api.exec`, `type=risk_policy`).

Note: `uaa-0016` was previously referenced inline as “follow-on work” in
`docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md`, but the decision and
constraints deserve a dedicated ADR.

## Related Docs

- Backlog:
  - `docs/backlog.json` (`uaa-0016`)
- Universal Agent API baselines:
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/extensions-spec.md` (owner doc for the core key)
- Planning pack (non-normative seam extraction):
  - `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/`

## Executive Summary (Operator)

ADR_BODY_SHA256: 37cf98a07880307dc2e156ca91a3c4ef5185db622ab4da3e48d5c7b74400567d

### Decision (draft)

- Introduce a new dangerous core extension key:
  - `agent_api.exec.external_sandbox.v1` (boolean; default `false`)
- The key is explicitly dangerous and MUST remain capability-gated per the extensions registry R0:
  - backends MUST NOT accept the key unless they advertise it in `capabilities()`, and
  - built-in backends SHOULD NOT advertise it by default; hosts must opt in explicitly via backend
    configuration.
- Meaning:
  - When `agent_api.exec.external_sandbox.v1 == true`, the host asserts it provides an external
    isolation boundary and requests the backend relax/disable internal guardrails (approvals,
    sandboxing, permissions) that would otherwise block unattended automation.
  - The backend MUST remain non-interactive (MUST NOT hang on prompts).
- Validation / contradiction rules (normative in `extensions-spec.md`):
  - value MUST be boolean; otherwise fail before spawn with `AgentWrapperError::InvalidRequest`.
  - if `agent_api.exec.external_sandbox.v1 == true` and `agent_api.exec.non_interactive == false`
    is explicitly requested (and both keys are supported), the backend MUST fail before spawn with
    `AgentWrapperError::InvalidRequest` (contradictory intent).
- Backend mapping intent (examples; details belong in backend docs/implementation):
  - Codex: map to `--dangerously-bypass-approvals-and-sandbox` (or an equivalent non-interactive
    combination of overrides).
  - Claude Code: map to `--dangerously-skip-permissions` (and, if required by the installed CLI
    version, `--allow-dangerously-skip-permissions`).

### Why

- Some orchestrators already provide strong isolation externally and need the CLI backend to avoid
  internal prompting/guardrails that conflict with unattended automation.
- Making the bypass explicit (and capability-gated) preserves safe-by-default posture for general
  Universal Agent API consumers while allowing explicitly sandboxed hosts to opt in.

## Problem / Context

The Universal Agent API is safe-by-default and fail-closed:

- extension keys are capability-gated and validated before spawn,
- backends avoid unbounded “argv pass-through”, and
- dangerous behavior must be explicitly requested.

However, some environments already enforce a sandbox boundary outside the CLI (e.g., containerized
orchestrators). For these hosts, internal approvals/sandbox/permissions guardrails may be both:

- incompatible with headless automation (prompting/hanging), and
- redundant given the external isolation boundary.

We need an explicit, cross-backend way to request this behavior without relying on implied
semantics (e.g. “non-interactive implies bypass”) or backend-specific ad-hoc keys.

## Goals

- Provide a single, explicit, versioned opt-in key for externally sandboxed execution posture.
- Preserve Universal Agent API posture:
  - fail-closed on unsupported keys,
  - validate before spawn, and
  - never hang on prompts.
- Keep built-in backends safe-by-default by requiring host-side enablement for capability
  advertising.

## Non-Goals

- Implementing or validating the external sandbox itself.
- Changing default safety posture when the key is absent.
- Introducing a generic “danger mode” umbrella key for multiple policies.
- Allowing `agent_api.exec.non_interactive` to imply dangerous bypass behavior.

## Alternatives Considered

1) **Implied behavior via `agent_api.exec.non_interactive`**
   - Rejected: violates “explicit dangerous knobs” posture and creates surprising behavior changes.

2) **Backend-specific keys only (`backend.<agent_kind>.*`)**
   - Rejected: prevents cross-backend orchestration and encourages ad-hoc drift.

3) **Unbounded CLI arg pass-through**
   - Rejected: makes capability gating meaningless and undermines deterministic validation.

## Validation Plan (draft)

- Land the owner-doc semantics in `docs/specs/universal-agent-api/extensions-spec.md`.
- Add backend tests that pin:
  - capability is not advertised by default,
  - unsupported key fails with `UnsupportedCapability` before any value/contradiction validation,
  - value type validation + contradiction handling fail before spawn, and
  - best-effort argv/builder mapping for Codex + Claude Code when enabled + requested.

