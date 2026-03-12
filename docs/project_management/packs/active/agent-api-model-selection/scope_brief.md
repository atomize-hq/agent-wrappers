# Scope brief — Universal model selection (`agent_api.config.model.v1`)

## Goal

Introduce one capability-gated, backend-neutral extension key for explicit model selection so callers can request
`--model <id>` behavior through `agent_api` without branching on backend-specific request types.

## Why now

Both built-in backends already expose explicit model-selection hooks, and the universal contract surface has already
identified model selection as a promotion-worthy shared capability. The remaining work is to pin the exact request
shape, validation posture, capability advertising, and backend mapping so orchestration code can rely on one stable key.

## Primary users + JTBD

- **Host integrators / orchestrators**: “Select a backend-specific model through one universal request field without
  importing backend crates or inventing backend-specific branching.”
- **Backend maintainers**: “Map one shared extension key to each CLI’s existing `--model` surface while keeping
  validation deterministic and runtime failures safe.”

## In-scope

- Universal extension key:
  - `agent_api.config.model.v1`
- Pinned v1 semantics:
  - string-only schema,
  - Unicode-whitespace trimming before validation and mapping,
  - non-empty trimmed value,
  - trimmed UTF-8 byte bound `1..=128`,
  - absence means no model override,
  - model id remains backend-owned and opaque.
- Built-in backend capability advertising for Codex and Claude Code once deterministic mapping is wired.
- Built-in backend mappings:
  - Codex `--model <trimmed-id>`
  - Claude Code `--model <trimmed-id>`
- Error posture:
  - unsupported capability fails per R0 before spawn,
  - invalid shape/bounds fail as `AgentWrapperError::InvalidRequest`,
  - runtime backend rejection of a syntactically valid model id fails as safe `AgentWrapperError::Backend`.
- Regression coverage for validation ordering, trimmed mapping, absence behavior, backend runtime rejection, and
  terminal error-event emission when a stream is open.

## Out-of-scope

- Defining a universal model catalog, enum, alias layer, or compatibility matrix.
- Standardizing secondary routing knobs such as Claude Code `--fallback-model`.
- Guaranteeing the same model id is accepted by multiple backends.
- Requiring wrappers to query upstream APIs or ship local registries to validate model availability.

## Capability inventory (implied)

- Capability id:
  - `agent_api.config.model.v1`
- Validation responsibilities:
  - R0 allowlist/capability gate occurs before backend-specific parsing,
  - value must be JSON string,
  - trimming occurs before emptiness and length checks,
  - trimmed value is what reaches backend argv/builder mapping.
- Backend mapping responsibilities:
  - absent key preserves default backend model behavior,
  - present valid key emits exactly one `--model <trimmed-id>` mapping,
  - the key alone cannot authorize fallback-model or any other side-effectful tuning knobs.
- Runtime failure handling:
  - backend-owned “unknown/unavailable/unauthorized model” outcomes remain runtime/backend errors,
  - error messages are safe/redacted,
  - if the run stream is already open, the backend emits exactly one terminal `Error` event with the same safe message.

## Required invariants (must not regress)

- **R0 fail-closed ordering**: unsupported `agent_api.config.model.v1` fails as
  `AgentWrapperError::UnsupportedCapability` before any schema validation or spawn behavior.
- **Deterministic pre-spawn validation**: non-string, empty-after-trim, and oversize values fail before spawn as
  `AgentWrapperError::InvalidRequest`.
- **Trimmed-value mapping**: built-in backends MUST pass the trimmed value, not the raw value, to argv/builder mapping.
- **Safe absence semantics**: when the key is absent, backends MUST NOT synthesize a model id and MUST NOT emit `--model`.
- **Opaque identifier posture**: wrappers MUST NOT pretend to own a universal model namespace or local authoritative catalog.
- **Safe runtime rejection**: backend-owned rejection of an accepted model id resolves as `AgentWrapperError::Backend`
  with safe/redacted messaging, plus one terminal `Error` event when an already-open stream must close in error.

## Success criteria

- A caller can send `agent_api.config.model.v1` through `AgentWrapperRunRequest.extensions` to either built-in backend.
- The backend capability set advertises `agent_api.config.model.v1` exactly when that backend deterministically supports
  the v1 mapping.
- Valid requests trim and map to the expected CLI/wrapper `--model <id>` behavior for both built-in backends.
- Invalid requests fail before spawn with stable `InvalidRequest` behavior.
- Absent requests preserve current backend defaults with no emitted `--model`.
- Runtime backend rejection stays backend-owned and safe, without introducing raw stderr leakage or fake universal errors.

## Constraints

- Public semantics must stay within the existing universal extension framework and error taxonomy.
- No wrapper-owned model registry or dynamic remote validation is allowed in v1.
- Existing builder/request APIs in `crates/codex` and `crates/claude_code` should be reused rather than bypassed.
- The implementation must preserve existing run/event lifecycle guarantees when backend rejection happens after stream open.

## External systems / dependencies

- Upstream CLIs / wrapper surfaces:
  - `codex exec --model <id> ...`
  - `claude --print --model <id> ...`
  - `crates/codex/src/builder/mod.rs`
  - `crates/claude_code/src/commands/print.rs`
- Canonical universal contracts:
  - `docs/specs/universal-agent-api/extensions-spec.md`
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`

## Known unknowns / risks

- **Runtime rejection string variability**: built-in CLIs may reject unknown or unauthorized model ids with unstable raw
  stderr wording. v1 therefore pins only safe/redacted backend error translation, not a universal user-facing message.
- **Validation locus**: the repo must choose a stable normalization point so both backends enforce the same trim/bounds
  semantics without duplicating drift-prone logic.
- **Advertising timing**: if capability ids are advertised before the mapping is fully wired, callers can observe false
  positives; advertising must land alongside working normalization + mapping.

## Assumptions (explicit)

- `docs/specs/universal-agent-api/extensions-spec.md` remains the canonical owner document for
  `agent_api.config.model.v1`, with ADR-0020 providing rationale and rollout framing.
- Built-in Codex and Claude Code backends will advertise `agent_api.config.model.v1` unconditionally once the
  implementation lands, because both already have deterministic local `--model` mapping surfaces.
- No additional backend-specific opt-in config is needed for this key because model selection is not a dangerous or
  state-mutating capability.
