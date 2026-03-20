# ADR-0020 — Universal model selection (`agent_api.config.model.v1`)
#
# Note: Run `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md`
# after editing to update the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft (rationale + rollout ADR; canonical semantics are already approved in the Universal Agent API specs and this ADR must stay synchronized to them)
- Date (UTC): 2026-03-12
- Owner(s): spensermcconnell

## Scope

- Define a single core extension key for backend-neutral model selection:
  - `agent_api.config.model.v1`
- Pin:
  - schema and absence semantics,
  - validation timing and bounds,
  - backend mapping for Codex and Claude Code, and
  - the v1 error/portability posture for backend-defined model identifiers.

This ADR corresponds to backlog item `uaa-0002` (`bucket=agent_api.config`, `type=extension_key`).

## Related Docs

- Backlog:
  - `docs/backlog.json` (`uaa-0002`)
- Prior bounded pass-through decision:
  - `docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md`
- Universal Agent API baselines:
  - `docs/specs/universal-agent-api/contract.md`
  - `docs/specs/universal-agent-api/run-protocol-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/extensions-spec.md` (owner doc for the core key)
- Backend mapping seams:
  - `crates/codex/src/builder/mod.rs`
  - `crates/claude_code/src/commands/print.rs`

## Canonical authority + sync workflow

- Normative authority for `agent_api.config.model.v1` lives in:
  - `docs/specs/universal-agent-api/extensions-spec.md` for schema, trimming, absence semantics, runtime-rejection posture, and backend mapping requirements
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` for the stable capability id registry entry, bucket placement, and capability-advertising posture
- ADR-0020 remains the rationale and rollout record until the implementation is accepted; it is not the owner doc for normative semantics.
- Reconciliation workflow when model-selection semantics change:
  - update the canonical spec docs first (`extensions-spec.md`, `capabilities-schema-spec.md`, and any affected backend contract doc)
  - update this ADR in the same change so its rationale, rollout notes, and related-doc pointers match the canonical spec state
  - run `make adr-fix ADR=docs/adr/0020-universal-agent-api-model-selection.md` before merging so the drift guard records the synchronized ADR body
- Sync ownership:
  - the ADR owner(s) above own keeping this ADR, the feature pack README, and the canonical specs aligned whenever `agent_api.config.model.v1` semantics or advertising rules change

## Executive Summary (Operator)

ADR_BODY_SHA256: 7c90cd711621e2c3676fc01a5f4b37c0f1e1db0e6f776c0619a425b8ad6f3a0d

### Decision (draft)

- Introduce a new core extension key:
  - `agent_api.config.model.v1`
- Schema:
  - JSON string
  - trim leading/trailing Unicode whitespace before validation and mapping
  - effective trimmed value is non-empty
  - trimmed length `1..=128` bytes (UTF-8)
- Default when absent:
  - no explicit model override; the backend MUST NOT emit `--model`, and the underlying CLI keeps
    its default model-selection behavior.
- Meaning:
  - the value is an opaque backend-consumable model identifier supplied through a universal key.
  - the Universal Agent API standardizes the request surface, not a shared cross-backend model
    catalog.
- Backend mapping:
  - Codex exec/resume: `codex ... --model <id> ...`
  - Codex fork: current app-server fork flows take the pinned safe backend rejection path because
    the app-server fork subset exposes no model transport field
  - Claude Code: `claude --print --model <id> ...`
- Validation posture:
  - R0 capability gating applies first.
  - v1 performs syntax/bounds validation before spawn, but does not require wrappers to ship or
    maintain an authoritative local model catalog.
  - pre-spawn `InvalidRequest` failures use the exact safe template
    `invalid agent_api.config.model.v1` and MUST NOT echo raw model ids.
  - If the provided identifier is syntactically valid but unknown/unavailable to the backend CLI,
    the backend MUST fail as a safe `AgentWrapperError::Backend` translation.

### Why

- Both built-in backends already expose materially identical `--model <id>` semantics.
- Model choice is a stable, high-value orchestration knob that does not need backend-specific
  branching.
- Treating the identifier as opaque keeps the universal contract durable while avoiding fragile
  wrapper-owned model registries.

## Problem / Context

Codex and Claude Code both support explicit model selection, and the wrapper crates already expose
it at the builder/request layer:

- Codex: `CodexClientBuilder::model(...)`
- Claude Code: `ClaudePrintRequest::model(...)`

Without a core extension key, callers must either:

- branch on `agent_kind` and construct backend-specific requests, or
- wait for future backend-specific pass-through keys that add unnecessary divergence for a clearly
  shared capability.

ADR-0016 already identified model selection as one of the few near-term knobs that satisfies the
promotion rubric for `agent_api.*`:

- shared semantic meaning across backends,
- small, stable schema,
- safe default when absent, and
- deterministic request-shape validation before spawn.

The remaining design question is how much semantic validation v1 should attempt. A universal enum
or wrapper-owned model registry would be brittle because:

- backend model catalogs evolve independently,
- availability can depend on backend version/account/provider state, and
- the wrapper repo should not need constant catalog churn just to preserve a stable request field.

## Goals

- Provide one capability-gated, backend-neutral key for selecting a model.
- Keep the schema minimal and stable.
- Preserve safe absence semantics by treating the key as optional.
- Avoid inventing a fake “universal model namespace” that the backends do not actually share.

## Non-Goals

- Defining a universal catalog of model names or aliases.
- Guaranteeing that the same model id is valid across different backends.
- Standardizing secondary routing knobs such as Claude Code `--fallback-model`.
- Requiring wrappers to proactively query remote APIs to validate model availability.

## Proposed Design (Draft)

### Core extension key

`agent_api.config.model.v1`

Owner:
- `docs/specs/universal-agent-api/extensions-spec.md`

Schema:
- Type: string
- Bounds:
  - the backend MUST trim leading/trailing Unicode whitespace before validation and mapping
  - after trimming, value MUST be non-empty
  - the trimmed value MUST be `<= 128` bytes (UTF-8)

Absence semantics:
- When absent, no model override is requested.
- The backend MUST preserve its existing default model behavior and MUST NOT synthesize a model id.

Meaning:
- The caller requests that the backend invoke its underlying CLI with the supplied model id.
- The string is opaque to the Universal Agent API. It is interpreted by the target backend only.
- The trimmed value is the effective model id for all v1 semantics.
- This key standardizes only model selection. By itself, it MUST NOT imply any additional
  cross-backend semantics such as fallback-model selection, reasoning tuning, summary/verbosity
  changes, permission-policy changes, or other secondary routing behavior.

### Validation and error model

Before spawn:
- If the capability id is unsupported, fail per R0 with `AgentWrapperError::UnsupportedCapability`.
- If the JSON value is not a string, or the trimmed string is empty, or the bound is exceeded, fail
  with `AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" }`.
- InvalidRequest messages for this key MUST use exactly `invalid agent_api.config.model.v1` and
  MUST NOT echo the raw model id.
- The backend MUST pass the trimmed value, not the raw untrimmed value, to the underlying CLI
  mapping.

After spawn / backend-owned validation:
- If the string is syntactically valid but the backend CLI rejects it as unknown, unavailable, or
  unauthorized, the backend MUST surface that as `AgentWrapperError::Backend` with a
  safe/redacted `message`.
- The backend MUST NOT embed raw backend stdout/stderr in that message.
- If this failure occurs after the backend has already returned an `AgentWrapperRunHandle` and the
  consumer-visible events stream is still open, the backend MUST emit exactly one terminal
  `AgentWrapperEventKind::Error` event with the same safe/redacted message before closing the
  stream.
- v1 does not require a pinned “unknown model” universal message because the wrappers do not own a
  stable, authoritative model registry.

This split preserves deterministic validation of request shape while avoiding false precision about
runtime model availability.

### Backend mapping

#### Codex

- CLI form:
  - exec/resume: `codex exec --model <id> ...`
  - fork: current app-server fork subset has no model transport field
- Implementation seam:
  - `crates/codex/src/builder/mod.rs` (`CodexClientBuilder::model(...)`)
- Normative scope:
  - `agent_api.config.model.v1` MUST map the effective model id to `--model <id>` for Codex
    exec/resume flows.
  - If a run selects `agent_api.session.fork.v1` and also includes an accepted
    `agent_api.config.model.v1` payload, the Codex backend MUST fail the run before any
    `thread/list` / `thread/fork` / `turn/start` request with
    `AgentWrapperError::Backend { message: "model override unsupported for codex fork" }`.
  - This key MUST NOT, by itself, authorize additional Universal Agent API behavior beyond model
    selection itself.

#### Claude Code

- CLI form: `claude --print --model <id> ...`
- Implementation seam:
  - `crates/claude_code/src/commands/print.rs` (`ClaudePrintRequest::model(...)`)
- Normative scope:
  - `agent_api.config.model.v1` MUST map the effective model id to `--model <id>`.
  - This key MUST NOT, by itself, authorize `--fallback-model` or any other additional print-mode
    override unless requested through a separate explicit key.

### Capability advertising

- A backend MUST advertise `agent_api.config.model.v1` only when it can deterministically map the
  key to its underlying CLI/session surface.
- For the current built-in backends, this is expected to be unconditional once implementation lands,
  because Claude Code can honor the key across its print/session argv flows and Codex has an
  explicit exec/resume mapping plus a pinned safe fork-rejection path.

## Alternatives Considered

1. Backend-specific keys only
   - Rejected: both backends already share the same user-facing meaning, so separate keys add
     needless branching and undermine the promotion rubric from ADR-0016.

2. A universal enum or shared model catalog
   - Rejected: model inventories and access rules churn independently of the wrapper contract.

3. Unbounded raw argv/config pass-through
   - Rejected: a dedicated, typed key is simpler, capability-gated, and consistent with the
     Universal Agent API fail-closed posture.

## Rollout / Compatibility

- Additive only. Callers that do not send the key are unchanged.
- Existing builder-level support in both backend crates lowers implementation risk; the primary
  remaining work is `agent_api` request validation, capability advertising, backend-owned session
  mapping docs, and mapping tests.

## Validation Plan (Authoritative for this ADR once Accepted)

- `make adr-check ADR=docs/adr/0020-universal-agent-api-model-selection.md`
- Land the owner-doc semantics in `docs/specs/universal-agent-api/extensions-spec.md`.
- When built-in advertising changes, run `cargo run -p xtask -- capability-matrix` in the same change and verify that
  `docs/specs/universal-agent-api/capability-matrix.md` publishes the expected `agent_api.config.model.v1` entry for
  the enabled built-in backends.
- Add backend tests proving:
  - unsupported key fails before spawn,
  - non-string / empty / oversize values fail before spawn with the exact safe template
    `invalid agent_api.config.model.v1`,
  - surrounding whitespace is trimmed before validation and argv/builder mapping,
  - accepted (pre-spawn-valid) requests emit the expected `--model <id>` argv/builder mapping for Codex
    exec/resume and Claude Code session flows, and
  - absent key does not emit `--model`, and
  - Codex fork rejects accepted model-selection inputs before any app-server request with the
    pinned safe backend message, and
  - backend runtime rejection of an accepted model id resolves as `AgentWrapperError::Backend`
    (with terminal `Error` event emission when a stream is open), and
  - post-handle runtime-rejection tests use dedicated fake backend scenarios rather than live model
    catalogs:
    - Codex: `fake_codex_stream_exec_scenarios_agent_api` scenario
      `model_runtime_rejection_after_thread_started`
    - Claude Code: `fake_claude_stream_json_agent_api` scenario
      `model_runtime_rejection_after_init`

## Decision Summary

`agent_api.config.model.v1` is promoted as a first-class core key because the semantics are
genuinely shared across Codex and Claude Code. The universal contract standardizes the request
surface and validation bounds, while treating the model identifier itself as an opaque backend-owned
string.
