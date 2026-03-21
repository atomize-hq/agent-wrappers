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

### `agent_api.exec.external_sandbox.v1` (boolean; dangerous)

Owner: this spec (`extensions-spec.md`).

Schema:
- Type: boolean
- Default when absent: `false`

Meaning:
- When `true`, the host asserts it provides an external isolation boundary and requests that the
  backend relax/disable internal approvals/sandbox/permissions guardrails that would otherwise
  block unattended automation.
- This key is explicitly dangerous and MUST NOT be implied by `agent_api.exec.non_interactive` or
  any other benign key.
- When `true`, the backend MUST remain non-interactive (MUST NOT hang on prompts).

Validation rules:
- Value MUST be a boolean; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- If this key is `true` and `agent_api.exec.non_interactive` is explicitly set to `false` in the
  same request (and both keys are supported per R0), the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest` (contradictory intent).
- If this key is `true`, the request MUST NOT include any backend-scoped exec-policy keys under
  `backend.<agent_kind>.exec.*` (ambiguous precedence). If such a key is present (and supported per
  R0), the backend MUST fail before spawn with `AgentWrapperError::InvalidRequest`.
  - Example backend exec-policy keys: `backend.codex.exec.approval_policy`,
    `backend.codex.exec.sandbox_mode`.

Observability / audit signal (v1, pinned):
- When `extensions["agent_api.exec.external_sandbox.v1"] == true` is accepted (capability is
  advertised and the request passes validation), the backend MUST emit exactly one safe
  `AgentWrapperEventKind::Status` warning event with:
  - `channel="status"`
  - `message="DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)"`
  - `data=None`
- Emission ordering (pinned):
  - The warning MUST be emitted before any `TextOutput` / `ToolCall` / `ToolResult` events for that
    run.
  - If the backend also advertises `agent_api.session.handle.v1`, the warning MUST be emitted
    before the session handle facet `Status` event.
  - The backend MUST preserve this ordering even if it buffers events for post-hoc emission (i.e.,
    the warning must still appear earlier in the consumer-visible stream).
- Non-emission cases (pinned):
  - If the key is absent or `false`, the backend MUST NOT emit this warning.
  - If the key is present but unsupported (fails R0) or invalid/contradictory (fails validation),
    the backend MUST NOT emit this warning.

Backend mapping requirements:
- Backends that advertise this key MUST:
  - ensure the underlying CLI/wrapper will not prompt (approvals/permissions prompts),
  - ensure any “internal sandbox required” checks are bypassed/disabled as required by that backend,
  - and remain deterministic (no “spawn then retry with different flags”).
- Built-in backends MUST NOT advertise this capability by default; it is intended for explicitly
  externally sandboxed hosts, and requires explicit opt-in via backend configuration (see
  `docs/specs/universal-agent-api/contract.md`, "Dangerous capability opt-in (external sandbox exec policy)").
- Concrete backend mapping contracts:
  - Codex: `docs/specs/codex-external-sandbox-mapping-contract.md`
  - Claude Code: `docs/specs/claude-code-session-mapping-contract.md`

### `agent_api.config.model.v1` (string)

Owner: this spec (`extensions-spec.md`).

Schema:
- Type: string
- Default when absent: no explicit model override

Meaning:
- When present, the backend MUST request that its underlying CLI/backend select the supplied
  backend-defined model identifier.
- The backend MUST normalize the supplied value by trimming leading and trailing Unicode whitespace
  before validation and mapping.
- The trimmed value is the effective model id for all v1 semantics.
- This key is orthogonal to `agent_api.session.resume.v1` and `agent_api.session.fork.v1`.
  Backends MUST preserve the same accepted effective model id across new-session, resume, and
  fork decision-making. A selected session flow MUST either apply that model id unchanged or take
  a pinned safe backend-rejection path owned by its backend contract; it MUST NOT silently ignore
  an accepted model-selection request for session-based flows.
- This key standardizes only model selection. It MUST NOT, by itself, imply additional
  cross-backend semantics such as:
  - fallback-model selection,
  - reasoning-effort / summary / verbosity changes,
  - permission / policy changes, or
  - any other secondary routing or tuning behavior.
- When absent, the backend MUST preserve its existing default model-selection behavior and MUST NOT
  synthesize or infer a model id.

Validation rules:
- Value MUST be a string; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" }`.
- After trimming, the value MUST be non-empty; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" }`.
- The trimmed value MUST be no more than 128 UTF-8 bytes; otherwise the backend MUST fail before
  spawn with `AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" }`.

Error message posture (v1, pinned):
- InvalidRequest messages for this key MUST use the single exact safe template:
  - `invalid agent_api.config.model.v1`
- InvalidRequest messages for this key MUST NOT echo the raw model id.
- Backends MUST reuse that exact template for non-string, empty-after-trim, and oversize failures;
  they MUST NOT invent a more specific InvalidRequest message shape for this key in v1.

Mapping requirements:
- The backend MUST pass the trimmed value, not the raw untrimmed value, to its underlying
  CLI/backend mapping.
- Built-in backends that advertise this key MUST map it as follows:
  - Codex exec/resume: emit `--model <trimmed-id>`.
  - Codex fork: the current pinned app-server v1 subset has no model-selection transport field on
    `thread/fork` or `turn/start`; the backend contract in
    `docs/specs/codex-app-server-jsonrpc-contract.md` therefore owns a deterministic pre-handle
    safe rejection path for accepted model-selection inputs on fork flows.
  - Claude Code: emit exactly one `--model <trimmed-id>` pair in the root-flags region, before any
    `--add-dir` group, session-selector flags, `--fallback-model`, and the final prompt token per
    `docs/specs/claude-code-session-mapping-contract.md`.
- Built-in backends MUST NOT treat this key, by itself, as authorizing additional backend-specific
  knobs outside model selection.
  - Codex: MUST NOT rely on this key as authorization for additional Universal Agent API semantics
    beyond model selection itself.
  - Claude Code: MUST NOT map this key to `--fallback-model` or any other additional print-mode
    override unless another explicit key defines that behavior.

Runtime rejection behavior (v1, normative):
- If this key passed R0 capability gating and pre-spawn validation, but the selected backend later
  determines that the requested model id cannot be honored at runtime, the backend MUST fail the
  run as:
  - `AgentWrapperError::Backend { message }`
- This includes backend outcomes such as:
  - unknown model id,
  - unavailable model id for the current backend/runtime/account/provider state, or
  - unauthorized access to the requested model id, or
  - a backend/session transport that cannot apply an accepted model id to the targeted run flow.
- The `message` MUST be safe/redacted and MUST NOT embed raw backend stdout/stderr.
- v1 does not pin a universal message string for model-selection failure.
- If such failure occurs after the backend has already returned an `AgentWrapperRunHandle` and the
  consumer-visible events stream is still open, the backend MUST emit exactly one terminal
  `AgentWrapperEventKind::Error` event with the same safe/redacted message before closing the
  stream.

### `agent_api.exec.add_dirs.v1` (object)

Owner: this spec (`extensions-spec.md`).

Schema:
- Type: object
- Required keys:
  - `dirs` (array of string)
- Unknown keys:
  - invalid in v1 (closed schema)
- Default when absent: no extra context directories are requested

Meaning:
- When present, the backend MUST request that its underlying CLI/backend include the supplied
  directory roots as additional context/file-access roots for that run.
- The backend MUST normalize each supplied entry by trimming leading and trailing Unicode
  whitespace before validation and mapping.
- The trimmed value is the effective directory entry for all v1 semantics.
- Entries MAY be absolute or relative.
- Relative entries MUST resolve against the run's effective working directory (see `contract.md`
  "Working directory resolution (effective working directory)").
- There is intentionally no containment requirement that keeps resolved directories under the
  effective working directory.
- This key is orthogonal to `agent_api.session.resume.v1` and `agent_api.session.fork.v1`.
  Session selection for those flows remains owned by `AgentWrapperRunRequest.extensions`; this key
  does not introduce any separate request field or alternate selector surface.
  Backends MUST preserve the same accepted effective add-dir set across new-session, resume, and
  fork decision-making. A selected session flow MUST either apply that set unchanged or take a
  pinned safe backend-rejection path owned by its backend contract; it MUST NOT silently ignore an
  accepted add-dir request for session-based flows.
- When absent, the backend MUST NOT synthesize any additional directories and MUST NOT emit any
  `--add-dir` flag or equivalent backend-specific override on behalf of this key.

Validation rules:
- Value MUST be an object; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- Unknown object keys MUST cause `AgentWrapperError::InvalidRequest` (closed schema for `.v1`).
- `dirs` MUST be present; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- `dirs` MUST be an array; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- `dirs` MUST contain at least 1 and at most 16 entries (`1..=16`); otherwise the backend MUST
  fail before spawn with `AgentWrapperError::InvalidRequest`.
- Each `dirs[i]` entry MUST be a string; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- After trimming, each `dirs[i]` entry MUST be non-empty; otherwise the backend MUST fail before
  spawn with `AgentWrapperError::InvalidRequest`.
- After trimming, each `dirs[i]` entry MUST be no more than 1024 UTF-8 bytes; otherwise the
  backend MUST fail before spawn with `AgentWrapperError::InvalidRequest`.
- Resolution/normalization algorithm (pinned):
  - If the trimmed entry is absolute, keep it absolute.
  - If the trimmed entry is relative, resolve it against the effective working directory.
  - Normalize the resulting path lexically using platform path rules sufficient to fold redundant
    separators and `.` / `..` segments.
  - The backend MUST NOT perform shell-style `~` expansion or environment-variable expansion for
    this key.
  - v1 does not require filesystem canonicalization or symlink resolution for this key.
- After resolution and lexical normalization, each effective path MUST exist and MUST be a
  directory before spawn; otherwise the backend MUST fail before spawn with
  `AgentWrapperError::InvalidRequest`.
- After resolution and lexical normalization, duplicate directories MUST be removed while
  preserving first-occurrence order.

Error message posture (v1, pinned):
- InvalidRequest messages for this key MUST be safe-by-default and MUST NOT echo raw path values.
- InvalidRequest messages for this key MUST use one of these exact safe templates:
  - `invalid agent_api.exec.add_dirs.v1`
  - `invalid agent_api.exec.add_dirs.v1.dirs`
  - `invalid agent_api.exec.add_dirs.v1.dirs[<i>]`
- `<i>` is the zero-based decimal index of the failing `dirs[i]` entry.
- Backends MAY reuse the same template for multiple failure classes within the same location, but
  MUST NOT invent any other InvalidRequest message shape for this key.

Mapping requirements:
- The backend MUST pass the normalized unique directory list, in order, to its underlying
  CLI/backend mapping.
- Built-in backends that advertise this key MUST map it as follows:
  - Codex exec/resume: emit one repeated `--add-dir <dir>` pair per normalized unique directory.
  - Codex fork: the current pinned app-server v1 subset has no add-dir transport field on
    `thread/fork` or `turn/start`; the backend contract in
    `docs/specs/codex-app-server-jsonrpc-contract.md` therefore owns a deterministic pre-handle
    safe rejection path for accepted add-dir inputs on fork flows.
  - Claude Code: emit one variadic `--add-dir <dir...>` argument group containing the normalized
    unique directories in order, placed before session-selector flags and before the final prompt
    token per `docs/specs/claude-code-session-mapping-contract.md`.
- A backend MUST advertise `agent_api.exec.add_dirs.v1` only when it has a deterministic contract
  for every run surface it exposes for this key: either a pinned mapping that honors the accepted
  directory list or a pinned backend-owned safe rejection path.
- For the current built-in backends, this capability is expected to be advertised unconditionally
  once implementation lands; support MUST NOT depend on per-run path contents.

Runtime rejection behavior (v1, normative):
- If this key passed R0 capability gating and pre-spawn validation, but the selected backend later
  determines that the requested directories cannot be honored by the installed CLI/runtime for that
  run, the backend MUST fail the run as:
  - `AgentWrapperError::Backend { message }`
- This includes runtime/backend-owned failures such as:
  - an installed CLI that rejects the required add-dir surface,
  - a backend flow that cannot apply accepted add-dir inputs to the targeted session transport, or
  - any other backend-owned inability to honor the accepted effective directory set.
- Backend-owned rejection paths for this key apply only to accepted inputs. Malformed, out-of-
  bounds, missing, or otherwise invalid add-dir payloads MUST still fail as
  `AgentWrapperError::InvalidRequest` before any backend-owned session-transport rejection path is
  considered.
- The `message` MUST be safe/redacted and MUST NOT embed raw backend stdout/stderr.
- v1 does not pin a universal runtime-rejection message string for add-dir failures.
- If the backend can determine that inability before returning an `AgentWrapperRunHandle`, it MUST
  return `AgentWrapperError::Backend { message }` directly.
- If the backend discovers that inability during asynchronous startup/preflight after it has
  already returned an `AgentWrapperRunHandle`, it MUST surface the failure through that handle even
  if the backend surface was never spawned:
  - `completion` MUST resolve as `Err(AgentWrapperError::Backend { message })`, and
  - if the consumer-visible events stream is still open, the backend MUST emit exactly one
    terminal `AgentWrapperEventKind::Error` event with the same safe/redacted `message` before
    closing the stream.
- If such failure occurs after the backend has already returned an `AgentWrapperRunHandle` and the
  consumer-visible events stream is still open, the backend MUST emit exactly one terminal
  `AgentWrapperEventKind::Error` event with the same safe/redacted message before closing the
  stream.

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
