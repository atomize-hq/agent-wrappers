# Concrete Remediation Log — `agent-api-explicit-cancellation`

Date (UTC): 2026-02-24

Input audit report:
- `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/concrete-audit.report.json`

Scope:
- Docs-only remediation (no code changes).
- Goal: close all issue IDs in the report by making the referenced docs fully concrete.

## Triage

Issues by severity:
- Blocker: CA-0001
- Critical: CA-0002, CA-0003
- Major: CA-0004, CA-0005, CA-0006, CA-0007

Buckets (fix strategy):
- A. Local clarification: CA-0003, CA-0004, CA-0005
- B. Code-defined contract: CA-0004, CA-0005, CA-0007 (timeout calibration), part of CA-0001
- C. Doc-defined standard: CA-0001, CA-0002, CA-0006, CA-0007
- D. External standard: (none used)
- E. Decision required: CA-0001, CA-0002, CA-0007

Decision log:
- `docs/decisions/concrete-remediation-decisions.md`

## Files changed

- `docs/specs/universal-agent-api/run-protocol-spec.md`
- `docs/specs/universal-agent-api/contract.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-2-agent-api-surface.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/seam.md`
- `docs/decisions/concrete-remediation-decisions.md`

## Issue-by-issue remediation

### CA-0001 — Explicit cancellation does not define required event-stream + completion gating behavior

Status: **Fixed**

Restated requirement:
- Pin how explicit cancellation interacts with DR-0012 completion gating and pin consumer-visible
  event-stream behavior after `cancel()` (including buffered events and cancel-handle lifetime).

Evidence used:
- Harness driver plan requires “close universal stream” + “stop forwarding” + “keep draining”:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md` L9-L14
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` L13-L27
- Code already pins DR-0012 opt-out semantics (completion gated on stream finality unless consumer drops `events`):
  - `crates/agent_api/src/run_handle_gate.rs` L1-L14, L27-L50, L77-L81
  - `crates/agent_api/src/backend_harness/runtime.rs` L32-L38, L66-L68 (stream finality signal and drain-on-drop posture)

Changes made:
- Updated `docs/specs/universal-agent-api/run-protocol-spec.md` to:
  - define stream finality + consumer opt-out and pin DR-0012 gating rules,
  - require consumer-visible `events` stream closure on explicit cancellation,
  - pin buffered (non-live) event handling on cancellation (drop not-yet-emitted buffered events),
  - pin cancel-handle orthogonality/lifetime (must still function even if `events`/handle dropped),
  - explicitly state whether cancellation is an exception to gating (it is **not**; see decision CRD-0001).

Decisions introduced:
- CRD-0001, CRD-0002 (see `docs/decisions/concrete-remediation-decisions.md`).

### CA-0002 — Cancellation outcome precedence is ambiguous for error completion and simultaneous races

Status: **Fixed**

Restated requirement:
- Define whether “completion resolves successfully” means `Ok` only vs any resolution, and pin
  precedence/tie-breaking for cancellation vs backend completion (including backend error completion).

Evidence used:
- Pack driver plan uses explicit race language and “cancel wins” semantics (but previously lacked tie-breaking):
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md` L22-L25
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` L22-L27

Changes made:
- Updated `docs/specs/universal-agent-api/run-protocol-spec.md` to:
  - define cancellation precedence against both `Ok(...)` and `Err(...)`,
  - specify that cancellation overrides backend error completion if cancellation is requested first,
  - pin tie-breaking (concurrent readiness) with “cancellation wins”.
- Updated `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`
  to match the pinned precedence rules.

Decisions introduced:
- CRD-0003 (see `docs/decisions/concrete-remediation-decisions.md`).

### CA-0003 — Pack-local SEAM-1 contract leaves `run_control` signature as `-> ...`

Status: **Fixed**

Restated requirement:
- Remove placeholders and pin the exact Rust signature/return type for gateway `run_control(...)`,
  plus concrete gateway error behavior (unknown backend + unsupported capability) with full error shapes.

Evidence used:
- Canonical contract already shows the exact signature shape:
  - `docs/specs/universal-agent-api/contract.md` (gateway `run_control` signature and error taxonomy)
- Pack’s own canonicalization slice calls out removal of `-> ...` placeholders:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-1-canonical-contracts.md` L28-L36

Changes made:
- Updated `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` to:
  - replace `-> ...` with the exact `Pin<Box<dyn Future<...>>>` signature,
  - pin gateway error behavior for UnknownBackend and UnsupportedCapability (full shapes + exact field values).

### CA-0004 — `AgentWrapperGateway::run_control` does not specify UnknownBackend error behavior

Status: **Fixed**

Restated requirement:
- Pin that `AgentWrapperGateway::run_control(...)` returns UnknownBackend when no backend is registered,
  and pin how the `agent_kind` string value is derived.

Evidence used:
- Existing `AgentWrapperGateway::run(...)` implementation in code derives the string via `as_str().to_string()`:
  - `crates/agent_api/src/lib.rs` L183-L198
- Pack implementation plan already assumes UnknownBackend for `run_control(...)`:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-2-agent-api-surface.md` L109-L115

Changes made:
- Updated `docs/specs/universal-agent-api/contract.md` to pin UnknownBackend behavior for
  `AgentWrapperGateway::run_control(...)`, including the `agent_kind` value source.

### CA-0005 — Fail-closed UnsupportedCapability examples omit required `agent_kind` field

Status: **Fixed**

Restated requirement:
- Remove partial-field `UnsupportedCapability` examples and pin the required `agent_kind` value
  (at minimum for the gateway path).

Evidence used:
- Canonical error shape includes `agent_kind` + `capability`:
  - `docs/specs/universal-agent-api/contract.md` (AgentWrapperError enum)
- Fail-closed capability gating for extensions already uses the full shape in the extensions spec:
  - `docs/specs/universal-agent-api/extensions-spec.md` L39-L46

Changes made:
- Updated `docs/specs/universal-agent-api/run-protocol-spec.md` to use the full error shape for
  explicit cancellation capability gating and to pin the gateway’s `agent_kind` value source.
- Updated pack docs to remove partial-field examples:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-2-agent-api-surface.md`
- Updated `docs/specs/universal-agent-api/contract.md` to remove a partial-field example in the
  “Extensions and capability gating” section (full shape with `agent_kind`).

### CA-0006 — Backend termination contract does not define minimum 'best-effort termination' behavior

Status: **Fixed**

Restated requirement:
- Define a minimum, testable “best-effort termination” contract for built-in backends (Codex + Claude Code),
  including idempotence/non-blocking constraints and failure handling.

Evidence used:
- Run protocol requires best-effort termination attempt on `cancel()`:
  - `docs/specs/universal-agent-api/run-protocol-spec.md` (explicit cancellation section)
- Pack SEAM-3 already identifies termination hook approach points (kill_on_drop, channel-close semantics):
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-3-backend-termination/slice-2-termination-hooks.md` L34-L39, L62-L67
- Existing wrapper code uses `kill_on_drop(true)` for spawned processes:
  - `crates/codex/src/exec/streaming.rs` (various call sites; not reprinted here)

Changes made:
- Updated `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md` to:
  - define “termination hook” and “best-effort termination”,
  - require idempotent + non-blocking termination hooks,
  - pin observable safety constraints and failure handling (no leaking; no non-pinned error strings),
  - clarify time bounds are defined by SEAM-4 pinned test parameters.

### CA-0007 — Test requirements rely on unspecified 'bounded/modest' timeouts and termination criteria

Status: **Fixed**

Restated requirement:
- Pin numeric timeouts and explicit pass/fail termination criteria, including numeric backpressure parameters.

Evidence used:
- Existing `agent_api` integration tests already use 1–3 second timeouts:
  - `crates/agent_api/tests/c1_codex_exec_policy.rs` L76-L87
  - `crates/agent_api/tests/c2_codex_stream_exec_parity.rs` L88-L116

Changes made:
- Updated SEAM-4 slice docs to pin numeric timeouts and criteria:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`
    - pinned `FIRST_EVENT_TIMEOUT = 1s`
    - pinned `CANCEL_TERMINATION_TIMEOUT = 3s`
    - pinned termination observation criteria: `events` reaches `None` and `completion` resolves within the timeout
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md`
    - pinned `FIRST_EVENT_TIMEOUT = 1s`
    - pinned `DROP_COMPLETION_TIMEOUT = 3s`
    - pinned `MANY_EVENTS_N = 200`
- Updated `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/seam.md`
  to reference the pinned timeouts (removed qualitative “bounded timeouts” phrasing).

Decisions introduced:
- CRD-0004 (see `docs/decisions/concrete-remediation-decisions.md`).

## Verification

- Re-ran the concrete lexical scan over the audited doc set after remediation.
  - Output: `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/concrete-audit.scan.after.json`
- Note: the lexical scan’s match count is not a pass/fail signal; closure is validated against the
  concrete issue IDs above.
