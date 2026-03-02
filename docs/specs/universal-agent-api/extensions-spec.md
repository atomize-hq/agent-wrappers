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
- **Effective working directory**: the run’s resolved working directory, as defined in
  `docs/specs/universal-agent-api/contract.md` ("Working directory resolution (effective working directory)").

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

Extension validation precedence (normative):

- When validating `request.extensions` (after prompt non-empty validation per
  `run-protocol-spec.md`), the backend MUST apply the R0 key gate before extension value validation
  or cross-key extension contradiction rules.
- If any extension key in `request.extensions` is unsupported, the backend MUST fail the run with
  `AgentWrapperError::UnsupportedCapability` (per R0) and MUST NOT attempt to return
  `AgentWrapperError::InvalidRequest` for extension value validation or cross-key extension
  contradiction rules for that request.
- Cross-key extension contradiction rules (e.g., mutual exclusivity) apply only after all extension
  keys in the request have passed R0 (i.e., are supported).

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
  - Claude Code: map to `--permission-mode bypassPermissions` (see
    `docs/specs/claude-code-session-mapping-contract.md`).

### `agent_api.session.resume.v1` (object)

Owner: this spec (`extensions-spec.md`).

Schema:
- Type: object
- Required keys:
  - `selector` (string): `"last"` | `"id"`
- Conditional keys:
  - If `selector == "id"`, `id` (string) MUST be present and MUST be non-empty (after trimming).
  - If `selector == "last"`, `id` MUST be absent.
- Default when absent: no session resume behavior (backend starts a new session per its defaults).

Meaning:
- When present, the backend MUST resume the targeted prior session/thread and treat
  `AgentWrapperRunRequest.prompt` as a follow-up prompt for that resumed session (i.e., this is
  “resume + send prompt”, not “resume with no new prompt”).
- `selector == "last"`:
  - Resume the backend’s most recent session/thread in the run’s effective working directory
    (see `contract.md`; backend-defined persistence store).
- `selector == "id"`:
  - Resume the session/thread identified by `id` (identifier format is backend-defined).

Note:
- Callers that need to discover a usable backend-defined session/thread id SHOULD observe the
  session handle facet when capability id `agent_api.session.handle.v1` is advertised (see
  `event-envelope-schema-spec.md`).
- Callers that use `selector == "last"` SHOULD provide a stable `AgentWrapperRunRequest.working_dir`.
  If `working_dir` is absent, `"last"` is scoped to the backend’s default effective working directory
  (which may be ephemeral, e.g. a temp dir).

Validation rules:
- Value MUST be an object; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- The request MUST NOT also include `agent_api.session.fork.v1`; if both are present and both keys
  are supported by the backend (per R0), the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- Unknown object keys MUST cause `AgentWrapperError::InvalidRequest` (closed schema for `.v1`).

Selection failure behavior (v1, normative):

- If `selector == "last"` and no prior session/thread exists in the effective working directory,
  the backend MUST fail the run (MUST NOT start a new session implicitly).
- If `selector == "id"` and the id is unknown/unresumable, the backend MUST fail the run (MUST NOT
  fall back to `"last"` or start a new session).
- The failure MUST be surfaced as:
  - `AgentWrapperError::Backend { message }`
  - with `message` pinned as:
    - `"no session found"` for `selector == "last"` (empty scope),
    - `"session not found"` for `selector == "id"` (unknown/unresumable id).
- The error `message` MUST be safe-by-default and MUST NOT embed raw backend output.
- Translation guardrails:
  - The pinned messages above MUST be used only when the backend indicates an actual “not found”
    outcome for the requested selector.
  - Backends MUST NOT infer selection failure solely from non-success exit status and/or absence of
    assistant output.
- Timing:
  - Backends SHOULD validate selection before spawning any long-lived backend process when the
    backend can check cheaply/deterministically (e.g., local store lookup).
  - If a backend cannot determine selection failure pre-spawn, it MUST still translate the
    backend’s “not found” outcome into the pinned `AgentWrapperError::Backend` message above and
    MUST NOT surface backend-specific stderr content.
- Event emission when an events stream exists:
  - If selection failure occurs after the backend has already returned an `AgentWrapperRunHandle`,
    the backend MUST emit exactly one terminal `AgentWrapperEventKind::Error` event with
    `event.message == <pinned message>` before closing the consumer-visible stream.

Backend mapping requirements:
- Each backend that advertises this key MUST document its concrete mapping in its backend
  contract/spec docs (examples):
  - Codex: map to `codex exec --json resume --last -` / `codex exec --json resume <id> -` (prompt on
    stdin; see `docs/specs/codex-wrapper-coverage-scenarios-v1.md`, Scenario 3).
  - Claude Code: map per `docs/specs/claude-code-session-mapping-contract.md`.

### `agent_api.session.fork.v1` (object)

Owner: this spec (`extensions-spec.md`).

Schema:
- Type: object
- Required keys:
  - `selector` (string): `"last"` | `"id"`
- Conditional keys:
  - If `selector == "id"`, `id` (string) MUST be present and MUST be non-empty (after trimming).
  - If `selector == "last"`, `id` MUST be absent.
- Default when absent: no fork behavior (backend starts a new session per its defaults).

Meaning:
- When present, the backend MUST fork a new session/thread from the targeted prior session/thread
  and treat `AgentWrapperRunRequest.prompt` as a follow-up prompt for the forked session.
- `selector == "last"`:
  - Fork from the backend’s most recent session/thread in the run’s effective working directory
    (see `contract.md`; backend-defined persistence store).
- `selector == "id"`:
  - Fork from the session/thread identified by `id` (identifier format is backend-defined).

Note:
- Callers that need to discover a usable backend-defined session/thread id SHOULD observe the
  session handle facet when capability id `agent_api.session.handle.v1` is advertised (see
  `event-envelope-schema-spec.md`).
- Callers that use `selector == "last"` SHOULD provide a stable `AgentWrapperRunRequest.working_dir`.
  If `working_dir` is absent, `"last"` is scoped to the backend’s default effective working directory
  (which may be ephemeral, e.g. a temp dir).

Validation rules:
- Value MUST be an object; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- The request MUST NOT also include `agent_api.session.resume.v1`; if both are present and both keys
  are supported by the backend (per R0), the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- Unknown object keys MUST cause `AgentWrapperError::InvalidRequest` (closed schema for `.v1`).

Selection failure behavior (v1, normative):

The fork key MUST follow the same selection-failure behavior as `agent_api.session.resume.v1`
(including pinned `AgentWrapperError::Backend` messages and the terminal `Error` event emission rule
when a stream exists).

Backend mapping requirements:
- Each backend that advertises this key MUST document its concrete mapping in its backend
  contract/spec docs (examples):
  - Codex: map to the `codex app-server` JSON-RPC surface:
    - resolve the fork source thread (for `selector == "last"`, via `thread/list` filtered by the
      effective working directory; see `contract.md`),
    - fork via `thread/fork`, and
    - send the follow-up prompt via `turn/start` on the forked thread.
  - Claude Code: map to `--fork-session` together with `--continue` / `--resume <id>` (see
    `docs/specs/claude-code-session-mapping-contract.md`).

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
