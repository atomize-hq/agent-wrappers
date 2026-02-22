# Spec — Universal Agent API Extensions (Core keys + ownership rules)

Status: Approved  
Approved (UTC): 2026-02-21  
Date (UTC): 2026-02-20  
Canonical location: `docs/specs/universal-agent-api/`

This spec defines the **canonical extension key registry and rules** for `AgentWrapperRunRequest.extensions`.

Goals:
- eliminate “implied” extension semantics spread across feature packs,
- ensure every extension has exactly one authoritative owner document, and
- make onboarding new CLI agent backends deterministic and contradiction-free.

Normative language: RFC 2119 requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Baselines (referenced; not duplicated)

- Universal Agent API contract:
  - `docs/specs/universal-agent-api/contract.md`
- Run protocol and validation timing:
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
- Capability id naming and extension gating requirement:
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`

## Definitions

- **Extension key**: a string key in `AgentWrapperRunRequest.extensions` (JSON value).
- **Core extension key**: a key under `agent_api.*` that is intended to be shared across many backends.
- **Backend extension key**: a key under `backend.<agent_kind>.*` that is meaningful only for a specific backend.
- **Owner doc**: the single authoritative spec/contract doc that defines:
  - schema (type/allowed values),
  - defaults and absence semantics,
  - validation rules and contradiction rules, and
  - mapping to any underlying CLI flags/config.

## Global rules (normative)

### R0 — Fail-closed capability gating

For every run, backends MUST validate `request.extensions` before spawning any backend process:

- For each key `k` in `request.extensions`:
  - If `backend.capabilities().contains(k)` is `false`, the backend MUST fail the run with:
    - `AgentWrapperError::UnsupportedCapability { agent_kind, capability: k }`.
  - If `backend.capabilities().contains(k)` is `true`, the backend MUST:
    - validate the JSON value type and allowed value constraints, and
    - apply defaults/absence semantics as defined by the owner doc for `k`.

This rule is the universal mechanism that makes extension onboarding scalable: extension keys are
declared in capabilities and validated deterministically per backend.

### R1 — Ownership (single source of truth)

- Every extension key MUST have exactly one owner doc.
- Core keys (`agent_api.*`) MUST be owned by this spec.
- Backend keys (`backend.<agent_kind>.*`) MUST be owned by that backend’s authoritative contract/spec
  documentation (e.g., a backend pack `contract.md`), and MUST NOT be defined here.

### R2 — Stability

- Once shipped, core extension key semantics are stable.
- Backend extension keys are stable per backend once shipped, but may be added over time.

## Core extension keys (normative registry)

### `agent_api.exec.non_interactive` (boolean)

Owner: this spec (`extensions-spec.md`).

Schema:
- Type: boolean
- Default when absent: `true`

Meaning:
- When `true`, a backend MUST configure its underlying CLI/wrapper to avoid interactive prompts
  that could hang automation (approvals/permissions prompts).
- When `false`, a backend MAY allow interactive behavior, but MUST remain deterministic with
  respect to validation and error reporting (no silent hangs that are avoidable with known flags).

Validation rules:
- Value MUST be a boolean; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.

Backend mapping requirements:
- Each backend that advertises this capability MUST document its concrete mapping in its backend
  contract/spec docs (examples):
  - Codex: map to an explicit approval policy that never prompts.
  - Claude Code: map to `--permission-mode bypassPermissions` (or equivalent).

## Adding new extension keys (process rules)

### Adding a new core key (`agent_api.*`)

1) Add the key and full semantics to this spec.
2) Update the Universal Agent API planning pack `spec_manifest.md` coverage matrix to assign the
   new surface to this spec.
3) Update any built-in backends that should support the key:
   - advertise the key in `capabilities()`
   - implement validation and mapping deterministically
   - add C2-style fake-binary tests if the key affects spawn behavior or safety.

### Adding a backend key (`backend.<agent_kind>.*`)

1) Define the key in the backend’s authoritative contract/spec docs (not in this spec).
2) The backend MUST advertise the key in `capabilities()`.
3) The backend MUST validate the key/value before spawn and apply defaults deterministically.
