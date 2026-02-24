# Concrete Audit Report

Generated at: 2026-02-24T14:55:55Z

## Summary
- Files audited: 28
- Issues: 7 total (blocker 1 / critical 2 / major 4 / minor 0)

### Highest-risk gaps
1. CA-0001 — Explicit cancellation does not define required event-stream + completion gating behavior
2. CA-0002 — Cancellation outcome precedence is ambiguous for error completion and simultaneous races
3. CA-0003 — Pack-local SEAM-1 contract leaves `run_control` signature as `-> ...`

### Files with highest issue density
- `docs/specs/universal-agent-api/run-protocol-spec.md`: 3
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`: 1
- `docs/specs/universal-agent-api/contract.md`: 1
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md`: 1
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`: 1

## Issues

### CA-0001 — Explicit cancellation does not define required event-stream + completion gating behavior
- Severity: blocker
- Category: behavior
- Location: `docs/specs/universal-agent-api/run-protocol-spec.md` L35-L74
- Excerpt: “`AgentWrapperRunHandle.completion` MUST NOT resolve until: 1) the underlying backend process has exited, and 2) the event stream has emitted all buffered events (if any) and has terminated.”
- Problem: The run protocol pins completion gating and then adds explicit cancellation, but it does not specify how cancellation affects (or is constrained by) the completion gating rule, nor does it pin what the consumer-visible event stream must do after `cancel()` (close immediately, drain buffered events, emit a final event, etc.). SEAM-2 plans to close the universal stream and keep draining internally, but that behavior is not stated normatively in the run protocol.
- Required to be concrete:
  - Specify whether explicit cancellation is an exception to the completion gating rule, or whether cancellation completion MUST still wait for underlying process exit and event-stream termination.
  - Specify the required consumer-visible event stream behavior after `AgentWrapperCancelHandle::cancel()` (e.g., MUST close immediately vs MAY continue emitting until termination).
  - Specify what happens to buffered (non-live) events on cancellation: emit remaining buffered events vs drop them, and how that interacts with the “no late events after completion” guarantee.
  - Specify whether `cancel()` must still function if the caller drops `events` and/or the `AgentWrapperRunHandle` after obtaining the cancel handle (cancellation orthogonality/lifetime).
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md` L9-L14
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` L15-L27

### CA-0002 — Cancellation outcome precedence is ambiguous for error completion and simultaneous races
- Severity: critical
- Category: behavior
- Location: `docs/specs/universal-agent-api/run-protocol-spec.md` L62-L74
- Excerpt: “If `cancel()` occurs before completion resolves successfully, `completion` MUST resolve to: `Err(AgentWrapperError::Backend { message })` where `message == "cancelled"`.”
- Problem: The explicit cancellation rule is conditioned on “before completion resolves successfully” without defining whether that means “before `Ok(...)`” or “before any completion (Ok/Err)”. The docs also use race language (“does not complete first” / “cancel wins the race”) without pinning tie-breaking when cancellation and completion become ready concurrently. Without precedence rules for cancellation vs backend error completion, implementers and tests must guess which error should win.
- Required to be concrete:
  - Define what “completion resolves successfully” means in terms of `Result` (`Ok` only vs any resolution).
  - Pin precedence rules for cancellation vs backend completion outcomes (`Ok` and `Err`), including tie-breaking when both become ready concurrently.
  - Specify whether cancellation is allowed/required to override a backend error completion, or whether backend errors always take precedence once produced.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` L29-L34
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` L23-L27
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md` L22-L25
  - `docs/adr/0014-agent-api-explicit-cancellation.md` L109-L116

### CA-0003 — Pack-local SEAM-1 contract leaves `run_control` signature as `-> ...`
- Severity: critical
- Category: contract
- Location: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` L5-L22
- Excerpt: “`AgentWrapperGateway::run_control(&self, agent_kind, request) -> ...`”
- Problem: This document claims to be “normative for this pack” but still contains an ellipsis placeholder for the `run_control(...)` return type and does not pin the full callable contract (exact signature, error cases, and full error shapes). Downstream seams and tests are intended to use CA-C01 as the pinned source of truth; leaving placeholders requires implementers to infer or cross-reference other docs.
- Required to be concrete:
  - Replace the `-> ...` placeholder with the exact Rust signature and return type for `AgentWrapperGateway::run_control(...)` (including the `Future<Output=...>` shape used in the canonical contract).
  - Pin the full set of gateway error behaviors for `run_control(...)` (at minimum: unknown backend and unsupported capability) with exact `AgentWrapperError` variant shapes.
  - Ensure the pack-local contract uses the full `UnsupportedCapability { agent_kind, capability }` shape, not a partial field list.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-1-canonical-contracts.md` L6-L18
  - `docs/specs/universal-agent-api/contract.md` L214-L217

### CA-0004 — `AgentWrapperGateway::run_control` does not specify UnknownBackend error behavior
- Severity: major
- Category: contract
- Location: `docs/specs/universal-agent-api/contract.md` L209-L217
- Excerpt: “This MUST return `AgentWrapperError::UnknownBackend` when no backend is registered for `agent_kind`.”
- Problem: The contract explicitly pins UnknownBackend behavior for `AgentWrapperGateway::run(...)` but does not pin the corresponding behavior for `AgentWrapperGateway::run_control(...)`. Other docs assume `run_control` will also fail with UnknownBackend when no backend is registered, but the canonical contract does not state it.
- Required to be concrete:
  - Specify whether `AgentWrapperGateway::run_control(...)` MUST return `AgentWrapperError::UnknownBackend` when no backend is registered for the requested kind (and if not, pin the alternative).
  - Pin the exact `AgentWrapperError::UnknownBackend { agent_kind: <...> }` `agent_kind` value source for the gateway path.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-2-agent-api-surface.md` L109-L115

### CA-0005 — Fail-closed UnsupportedCapability examples omit required `agent_kind` field
- Severity: major
- Category: contract
- Location: `docs/specs/universal-agent-api/run-protocol-spec.md` L62-L66
- Excerpt: “`run_control(...)` MUST fail-closed with `AgentWrapperError::UnsupportedCapability { capability: "agent_api.control.cancel.v1" }`.”
- Problem: The normative cancellation capability-gating text uses an `UnsupportedCapability` example that omits `agent_kind`, but the canonical error shape includes `{ agent_kind, capability }`. Omitting `agent_kind` leaves the required field value underspecified and encourages inconsistent examples across docs (some use the full shape, others omit it).
- Required to be concrete:
  - Update the explicit cancellation capability-gating rule to specify the full error shape: `UnsupportedCapability { agent_kind: <...>, capability: "agent_api.control.cancel.v1" }`.
  - Pin the required `agent_kind` value for the gateway path (e.g., derived from the requested `AgentWrapperKind`).
  - Align pack-local contract and implementation plan docs to avoid partial-field examples for `UnsupportedCapability`.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/specs/universal-agent-api/contract.md` L155-L160
  - `docs/specs/universal-agent-api/extensions-spec.md` L43-L46
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` L17-L22
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-2-agent-api-surface.md` L111-L114

### CA-0006 — Backend termination contract does not define minimum 'best-effort termination' behavior
- Severity: major
- Category: behavior
- Location: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md` L5-L15
- Excerpt: “Built-in backends MUST support best-effort termination of the spawned CLI process.”
- Problem: The backend termination contract is stated only as “best-effort termination” without a minimum, testable definition (what action is required, whether it must be non-blocking, how failures are handled, and what time bounds apply). The notes mention `kill_on_drop(true)` as an approach, but are explicitly informative and not a concrete contract.
- Required to be concrete:
  - Define the minimum required termination action(s) that satisfy “best-effort termination” for built-in backends (Codex and Claude Code).
  - Specify whether termination hooks must be idempotent and non-blocking, and whether they must avoid introducing deadlocks/backpressure stalls.
  - Specify how termination failures (e.g., kill signal fails or is unsupported) are surfaced or handled in terms of observable completion/event behavior.
  - Pin any time-bound expectations needed for tests to be meaningful (even if platform-dependent), or explicitly state that time bounds are test-defined and must be pinned in SEAM-4.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L68-L70
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md` L7-L10

### CA-0007 — Test requirements rely on unspecified 'bounded/modest' timeouts and termination criteria
- Severity: major
- Category: testing
- Location: `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md` L6-L32
- Excerpt: “asserts best-effort termination (completion resolves within a bounded timeout)”
- Problem: The SEAM-4 test plan uses qualitative timeout language (“bounded timeout”, “modest (seconds, not minutes)”) without pinning concrete values or pass/fail criteria. This makes the acceptance criteria underspecified and risks flaky CI or insufficient regression coverage.
- Required to be concrete:
  - Pin numeric timeout values (or a bounded range) for the explicit cancellation integration test and the drop regression test, including any platform-specific adjustments.
  - Define the exact pass/fail criteria used to treat the fake process as “terminated” (e.g., completion resolves, process exit observed, event stream closed).
  - Pin any numeric parameters used for backpressure scenarios (e.g., the exact N events to emit) if they are relied upon for regression detection.
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md` L7-L13
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/seam.md` L28-L29
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md` L11-L14
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md` L53-L54

## Audited files
- docs/adr/0014-agent-api-explicit-cancellation.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/README.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/decision_register.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/scope_brief.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam_map.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/seam.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-1-canonical-contracts.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-2-agent-api-surface.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/seam.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-2-harness-control-entrypoint.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-3-backend-termination/seam.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-3-backend-termination/slice-1-backend-adoption.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-3-backend-termination/slice-2-termination-hooks.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/seam.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threading.md
- docs/specs/universal-agent-api/README.md
- docs/specs/universal-agent-api/capabilities-schema-spec.md
- docs/specs/universal-agent-api/contract.md
- docs/specs/universal-agent-api/extensions-spec.md
- docs/specs/universal-agent-api/run-protocol-spec.md
- docs/wi/wi-0002-explicit-cancellation-api.md
