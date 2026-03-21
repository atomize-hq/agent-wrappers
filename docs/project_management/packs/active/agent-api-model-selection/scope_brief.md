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
  - Codex exec/resume `--model <trimmed-id>`
  - Codex fork pinned safe rejection:
    `AgentWrapperError::Backend { message: "model override unsupported for codex fork" }`
  - Claude Code `--model <trimmed-id>`
- Error posture:
  - unsupported capability fails per R0 before spawn,
  - invalid shape/bounds fail as `AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" }`,
  - InvalidRequest messages for this key MUST NOT echo the raw model id,
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
  - canonical registry entry:
    `docs/specs/universal-agent-api/capabilities-schema-spec.md` (`agent_api.config.*` bucket; stable capability id)
  - canonical owner doc for schema/defaults/mapping:
    `docs/specs/universal-agent-api/extensions-spec.md`
- Validation responsibilities:
  - R0 allowlist/capability gate occurs before backend-specific parsing,
  - value must be JSON string,
  - trimming occurs before emptiness and length checks,
  - trimmed value is what reaches backend argv/builder mapping, and
  - invalid requests use the single exact safe template `invalid agent_api.config.model.v1`.
- Backend mapping responsibilities:
  - absent key preserves default backend model behavior,
  - present valid key emits exactly one `--model <trimmed-id>` mapping,
  - the key alone cannot authorize fallback-model or any other side-effectful tuning knobs.
- Runtime failure handling:
  - backend-owned “unknown/unavailable/unauthorized model” outcomes remain runtime/backend errors,
  - backend/session transports that cannot apply the accepted model id take a pinned safe backend
    rejection path,
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
- **No silent session drift**: accepted model-selection inputs either survive into resume/fork flows unchanged or take
  a pinned safe backend-rejection path owned by the backend contract docs.

## Success criteria

- A caller can send `agent_api.config.model.v1` through `AgentWrapperRunRequest.extensions` to either built-in backend.
- The backend capability set advertises `agent_api.config.model.v1` exactly when that backend deterministically supports
  the v1 mapping.
- Valid requests trim and map to the expected CLI/wrapper `--model <id>` behavior for both built-in backends.
- Codex fork rejects accepted model-selection inputs before any app-server request with the pinned safe backend
  message.
- Invalid requests fail before spawn with stable `InvalidRequest` behavior.
- Absent requests preserve current backend defaults with no emitted `--model`.
- Capability publication is owned by SEAM-2: any change that flips built-in advertising for
  `agent_api.config.model.v1` MUST regenerate `docs/specs/universal-agent-api/capability-matrix.md` via
  `cargo run -p xtask -- capability-matrix` in the same change. Until that implementation change lands, the
  generated matrix may legitimately have no `agent_api.config.model.v1` row; reviewers MUST treat
  `docs/specs/universal-agent-api/capabilities-schema-spec.md` as the canonical registry anchor and only expect a
  matrix row in the change set that actually enables built-in advertising.
- Runtime backend rejection stays backend-owned and safe, without introducing raw stderr leakage or fake universal errors.

## Constraints

- Public semantics must stay within the existing universal extension framework and error taxonomy.
- No wrapper-owned model registry or dynamic remote validation is allowed in v1.
- Existing builder/request APIs in `crates/codex` and `crates/claude_code` MUST be reused rather than bypassed.
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
- Canonical backend mapping contracts:
  - `docs/specs/codex-streaming-exec-contract.md`
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
  - `docs/specs/claude-code-session-mapping-contract.md`

## Pinned execution decisions

- **SEAM-1 start condition**: canonical owner-spec text is already pinned. SEAM-1 is complete once the pack and
  ADR-0020 are synchronized and no unresolved delta remains in related universal specs; downstream seams do not wait
  on a fresh schema decision.
- **Normalization locus**: v1 normalization MUST be implemented as one shared helper in
  `crates/agent_api/src/backend_harness/normalize.rs`. Backend-local mirrored parsers are not permitted for this key.
- **Shared helper interface**:
  - SEAM-2 owns one backend-neutral entrypoint in `crates/agent_api/src/backend_harness/normalize.rs` that reads only
    `request.extensions.get("agent_api.config.model.v1")` after the R0 allowlist/capability gate has accepted the key.
  - The helper contract is `Result<Option<String>, AgentWrapperError>` with these pinned outcomes:
    - `Ok(None)` when the key is absent.
    - `Ok(Some(trimmed_model_id))` when the JSON value is a string whose trimmed UTF-8 byte length is `1..=128`.
    - `Err(AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" })` when the value is
      present but non-string, empty after trimming, or longer than 128 UTF-8 bytes after trimming.
  - The returned `String` is already trimmed and is the only value backend seams may forward downstream; backend code
    MUST NOT re-trim, re-parse, or read the raw extension payload again.
- **Builder/request consumption order**:
  - Codex policy extraction consumes `Some(trimmed_model_id)` by calling `CodexClientBuilder::model(trimmed_model_id)`;
    `None` means no `.model(...)` call.
  - Claude policy extraction consumes `Some(trimmed_model_id)` by calling `ClaudePrintRequest::model(trimmed_model_id)`;
    `None` means no `.model(...)` call.
  - Existing builder/request argv implementations remain the only place that emits `--model`; backend seams MUST reuse
    those surfaces instead of hand-writing a second `--model` mapping from raw `request.extensions`.
- **Capability-matrix handoff**: SEAM-2 owns matrix regeneration in the same change that updates built-in capability
  advertising. SEAM-5 consumes that artifact for regression assertions, and WS-INT reruns
  `cargo run -p xtask -- capability-matrix`; a stale diff is merge-blocking.
- **Single-parser enforcement**:
  - SEAM-2 MUST land shared helper tests in `crates/agent_api/src/backend_harness/normalize.rs` for absence, non-string,
    whitespace-only, oversize-after-trim, and trimmed-success cases.
  - SEAM-3/4 tests MUST prove the backend wiring consumes the helper output unchanged and emits `--model` through the
    existing builder/argv order documented by the canonical backend specs.
  - Code review for SEAM-2/3/4 is incomplete until the diff shows no new direct parsing of
    `agent_api.config.model.v1` outside `crates/agent_api/src/backend_harness/normalize.rs`.
- **Runtime-rejection fixtures**:
  - Codex uses `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs` with a dedicated
    `model_runtime_rejection_after_thread_started` scenario that emits `thread.started` before the terminal failure.
  - Claude Code uses `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs` with a dedicated
    `model_runtime_rejection_after_init` scenario that emits `system init` before the terminal failure.
  - Both backend tests MUST assert the same safe message in completion and in the final
    `AgentWrapperEventKind::Error` event, and MUST prove that raw model ids/stdout/stderr do not leak.

## Risks

- **Runtime rejection string variability**: built-in CLIs may reject unknown or unauthorized model ids with unstable raw
  stderr wording. v1 therefore pins only safe/redacted backend error translation, not a universal user-facing message.
- **Advertising timing**: if capability ids are advertised before the mapping is fully wired, callers can observe false
  positives; advertising must land alongside working normalization + mapping.
- **Codex fork transport gap**: the current app-server fork subset exposes no model field, so fork support depends on
  keeping the pinned pre-handle rejection contract aligned with the universal capability semantics.

## Assumptions (explicit)

- `docs/specs/universal-agent-api/extensions-spec.md` remains the canonical owner document for
  `agent_api.config.model.v1`, with ADR-0020 providing rationale and rollout framing.
- Built-in Codex and Claude Code backends will advertise `agent_api.config.model.v1` unconditionally once the
  implementation lands, because Claude Code can honor the key across its print/session argv flows and Codex has an
  explicit exec/resume mapping plus a pinned safe fork-rejection path.
- No additional backend-specific opt-in config is needed for this key because model selection is not a dangerous or
  state-mutating capability.
