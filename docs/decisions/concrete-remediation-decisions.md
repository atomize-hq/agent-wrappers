# Concrete remediation decisions

Dates (UTC): 2026-02-24, 2026-02-28, 2026-03-04

Scope: Documentation-only remediation for `agent_api` concrete-audit gaps, based on:
- `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/concrete-audit.report.json`
- `concrete-audit.report.json` (session semantics pack + UAA specs)
- `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/logs/concrete-audit.report.json`

This document records concrete decisions introduced where the audit required an explicit choice and
no single authoritative source fully pinned the behavior.

## CRD-0001 — Explicit cancellation is not an exception to completion gating

**Decision**

Explicit cancellation MUST still respect DR-0012 completion gating:
- `completion` MUST NOT resolve until the underlying backend process has exited, and
- unless the consumer drops `events` (opt-out), the consumer-visible `events` stream is final (`None`).

**Context**

CA-0001 required specifying whether explicit cancellation is an exception to the completion gating
rule, and how cancellation interacts with the “no late events after completion” guarantee.

**Chosen spec**

Pinned in `docs/specs/universal-agent-api/run-protocol-spec.md` under:
- “Relationship between `completion` and the event stream (DR-0012 / v1, normative)”
- “Explicit cancellation semantics (v1, normative)” → “Completion gating”

**Rationale**

- Keeps `completion` as the reliable “the run is fully done (process exit reached)” signal, which
  lets tests treat `completion` resolution as the termination observation point.
- Preserves the “no late events after completion” invariant for consumers that keep `events` alive.

**Implications**

- Implementations MUST request best-effort termination on `cancel()` and resolve completion only
  after process exit.
- Consumers may observe stream finality before completion in cancellation paths.

## CRD-0002 — Consumer-visible event stream closes on cancellation and buffered events are dropped

**Decision**

After cancellation is requested, the backend MUST:
- stop forwarding additional events to the consumer-visible `events` stream, and
- close the consumer-visible `events` stream (consumer can observe `None`).

If the backend buffers events for post-hoc emission (non-live), any buffered events not yet emitted
MUST be dropped (MUST NOT be flushed to the consumer after cancellation).

**Context**

CA-0001 required pinning consumer-visible event-stream behavior and buffered-event handling after
`cancel()`.

**Chosen spec**

Pinned in `docs/specs/universal-agent-api/run-protocol-spec.md` under:
- “Explicit cancellation semantics (v1, normative)” → “Consumer-visible event stream behavior after `cancel()`”

**Rationale**

- Aligns with the SEAM-2 driver plan (“stop forwarding” + “close stream”) and provides deterministic
  behavior for orchestrators.
- Avoids ambiguous “late events after completion” interactions in cancellation flows.

**Implications**

- Implementations MUST separate internal draining (to avoid deadlocks) from consumer-visible
  forwarding.
- Consumers must not rely on receiving post-hoc buffered events after requesting cancellation.

## CRD-0003 — Cancellation outcome precedence and tie-breaking

**Decision**

If cancellation is requested before `completion` resolves, the completion outcome MUST be the pinned
cancellation error:
- `Err(AgentWrapperError::Backend { message: "cancelled" })`

This MUST override any backend `Ok(...)` or `Err(...)` completion that would otherwise resolve after
cancellation is requested.

If `completion` resolves first, cancellation is a no-op and MUST NOT change the already-resolved
value.

If cancellation and completion become ready concurrently, cancellation wins.

**Context**

CA-0002 required precedence rules for cancellation vs backend completion outcomes and explicit
tie-breaking for simultaneous readiness.

**Chosen spec**

Pinned in:
- `docs/specs/universal-agent-api/run-protocol-spec.md` → “Explicit cancellation semantics” → “Completion outcome and precedence (pinned)”
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` → “Completion outcome and precedence (pinned)”

**Rationale**

- Ensures deterministic cancellation behavior for orchestrators and tests.
- Avoids flake from race ambiguity and avoids exposing backend-specific kill/exit error shapes.

**Implications**

- Implementations should use cancellation-biased race resolution when cancellation and backend
  completion are simultaneously ready.

## CRD-0004 — SEAM-4 pinned timeouts and parameters

**Decision**

SEAM-4 tests use the following pinned parameters in v1:

- `FIRST_EVENT_TIMEOUT = 1s`
- `CANCEL_TERMINATION_TIMEOUT = 3s`
- `DROP_COMPLETION_TIMEOUT = 3s`
- `MANY_EVENTS_N = 200`

No platform-specific adjustment in v1 (same values on all supported platforms).

**Context**

CA-0007 required numeric timeout values, explicit pass/fail termination criteria, and pinned numeric
parameters for backpressure/drain regression tests.

**Chosen spec**

Pinned in:
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md`

**Rationale**

- Keeps CI-friendly “seconds, not minutes” budgets while matching existing repo patterns (other
  `agent_api` integration tests commonly use ~1–3 second `tokio::time::timeout` windows).

**Implications**

- Cancellation/termination implementations must satisfy these timeouts in CI on supported platforms.

## CRD-0005 — Session selection failure is an error (no implicit start-new)

**Decision**

For `agent_api.session.resume.v1` and `agent_api.session.fork.v1`:
- If selection fails (`selector=="last"` with no prior session in scope; or `selector=="id"` not found),
  the backend MUST fail the run and MUST NOT start a new session implicitly.

**Context**

CA-0002 required a concrete error model for “no session” / “id not found” so implementers can write
deterministic tests and built-in backends don’t diverge.

**Chosen spec**

Pinned in `docs/specs/universal-agent-api/extensions-spec.md` under:
- `agent_api.session.resume.v1` → “Selection failure behavior (v1, normative)”
- `agent_api.session.fork.v1` → “Selection failure behavior (v1, normative)”

**Rationale**

Resume/fork keys are explicit user intent; silently starting a new session would violate caller
expectations and make failure-path tests non-deterministic.

**Implications**

- Built-in backends must translate backend-specific “not found” outcomes into a universal error.
- Orchestrators can treat selection failures as actionable errors rather than “maybe started new”.

## CRD-0006 — Pinned selection-failure error variant + messages

**Decision**

Selection failures MUST be surfaced as:
- `AgentWrapperError::Backend { message }`
- with `message` pinned as:
  - `"no session found"` for `selector=="last"`,
  - `"session not found"` for `selector=="id"`.

If selection failure occurs after an `AgentWrapperRunHandle` is returned, the backend MUST emit
exactly one terminal `AgentWrapperEventKind::Error` event with the same pinned message before
closing the stream.

**Context**

CA-0002 required choosing a concrete universal error variant/message and specifying stream emission
requirements when an events stream exists.

**Chosen spec**

Pinned in `docs/specs/universal-agent-api/extensions-spec.md` (same section as CRD-0005).

**Rationale**

- Keeps errors safe-by-default and testable without depending on backend stderr content.
- Provides a single universal behavior for both built-in backends and future adapters.

**Implications**

- Tests should assert the exact message strings, not backend-specific outputs.
- Backends must not embed raw backend lines in `AgentWrapperEvent.data` / error messages.

## CRD-0007 — Codex app-server “last” selection algorithm and tie-breaker

**Decision**

For Codex fork via app-server `thread/list` (selector `"last"`), “most recent” is selected by the
maximum tuple:
- `(updatedAt, createdAt, id)` (lexicographic; largest wins),
aggregated across all pages (follow `nextCursor` until `null`).

**Context**

CA-0001 required a concrete “last” selection algorithm (including tie-breaking) and a concrete
`thread/list` contract, rather than “likely use listing”.

**Chosen spec**

Pinned in `docs/specs/codex-app-server-jsonrpc-contract.md` under:
- `thread/list` → “Selection algorithm (pinned, deterministic)”

**Rationale**

- Avoids depending on unspecified server-side ordering semantics.
- Makes fake-server and integration tests deterministic and backend-agnostic.

**Implications**

- Implementations must page (or otherwise guarantee equivalent coverage) to avoid missing newer threads.

## CRD-0008 — Codex streaming resume requires a control + env-overrides entrypoint

**Decision**

`agent_api` Codex resume MUST use a control-capable `crates/codex` streaming entrypoint that:
- accepts per-run env overrides, and
- returns a termination handle (`ExecStreamControl.termination`).

Pinned API name/signature:
- `codex::CodexClient::stream_resume_with_env_overrides_control(request: codex::ResumeRequest, env_overrides: &BTreeMap<String, String>) -> Result<codex::ExecStreamControl, codex::ExecStreamError>`

**Context**

CA-0003 required pinning the concrete wrapper API + CLI prompt plumbing needed for deterministic
tests and to preserve `agent_api` cancellation/env semantics parity with `exec`.

**Chosen spec**

Pinned in:
- `docs/project_management/packs/active/agent-api-session-semantics/threading.md` → `SA-C05`
- `docs/project_management/packs/active/agent-api-session-semantics/seam-3-session-resume-extension-key.md`

**Rationale**

Existing `stream_resume` lacks a termination handle and per-run env overrides; `agent_api` requires
both to meet universal cancellation semantics and config precedence rules.

**Implications**

- A small additive API change is required in `crates/codex` before Codex resume can be shipped in `agent_api`.

## CRD-0009 — Codex app-server fork notification mapping is metadata-only and safety-first

**Decision**

For Codex fork via app-server (`thread/*` + `turn/start`), notification → `AgentWrapperEvent` mapping is pinned as:

- `agentMessage/delta` and reasoning delta notifications → `AgentWrapperEventKind::TextOutput` (bounded; split if needed).
- `item/started` → `AgentWrapperEventKind::ToolCall` (metadata-only).
- `item/completed` → `AgentWrapperEventKind::ToolResult` (metadata-only).
- `turn/started` / `turn/completed` → `AgentWrapperEventKind::Status` (safe message optional).
- `error` → `AgentWrapperEventKind::Error` with `message` derived from the bounded `error.message`.

Raw backend payloads MUST NOT be embedded in `AgentWrapperEvent.data`.

**Context**

CA-0001 required enumerating which app-server notifications must be mapped and defining the exact
mapping into universal event kinds/fields while preserving the Universal Agent API safety posture.

**Chosen spec**

Pinned in `docs/specs/codex-app-server-jsonrpc-contract.md` under:
- “Notifications → Universal Agent API events (pinned minimum)”

**Rationale**

- Matches the repo’s v1 safety posture (no raw backend lines/payloads in `data`).
- Keeps fork integration tests deterministic while still providing useful high-level event kinds.

**Implications**

- Tool details must be carried via the structured tools facet (`agent_api.tools.structured.v1`) in
  future work, not via raw payload embedding.

## CRD-0010 — Non-interactive approval request fails fast with a pinned safe message

**Decision**

When `agent_api.exec.non_interactive == true` and an approval request is observed during a Codex
app-server-backed flow, the backend MUST fail the run with:

- `AgentWrapperError::Backend { message: "approval required" }`

If an events stream exists and is still open, the backend MUST emit exactly one terminal
`AgentWrapperEventKind::Error` event with `event.message == "approval required"` before closing the
stream.

**Context**

CA-0003 required the app-server contract to define what constitutes an “approval request” on the
wire and to pin deterministic fail-fast behavior (including a safe error message) under
non-interactive configuration.

**Chosen spec**

Pinned in `docs/specs/codex-app-server-jsonrpc-contract.md` under:
- “Non-interactive safety (`agent_api.exec.non_interactive`) (pinned)” → “Fail-fast handling (non-interactive, pinned)”

**Rationale**

- Provides a single safe-by-default string suitable for contract-based tests.
- Avoids embedding raw server approval payloads in universal errors/events.

**Implications**

- Tests MUST assert the exact string `"approval required"` when exercising this failure mode.

## CRD-0011 — External sandbox requests emit a safe Status warning

**Decision**

When `extensions["agent_api.exec.external_sandbox.v1"] == true` is accepted (capability advertised
and request passes validation), the backend MUST emit exactly one safe
`AgentWrapperEventKind::Status` warning event with:
- `channel="status"`
- `message="DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)"`
- `data=None`

Emission timing is pinned:
- the warning MUST be emitted before any `TextOutput` / `ToolCall` / `ToolResult` events for that
  run.

**Context**

Concrete audit CA-0005 required deciding whether this dangerous key has an operator-visible signal
and, if so, pinning its minimal schema/fields and emission point.

**Chosen spec**

Pinned in:
- `docs/specs/universal-agent-api/extensions-spec.md` under:
  - `agent_api.exec.external_sandbox.v1` → “Observability / audit signal (v1, pinned)”
- (Planning summary copy): `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/seam-1-external-sandbox-extension-key.md`

**Rationale**

- This key explicitly requests relaxation of safety guardrails; a stable warning makes that posture
  change visible to orchestrators without requiring new facets/schemas.
- `Status` is already a bounded, safe surface per `event-envelope-schema-spec.md`.

**Implications**

- Implementations must inject/forward exactly one stable warning per accepted run and keep it free
  of raw backend output.
- Tests should pin both presence (when requested + accepted) and absence (when absent/false).

## CRD-0012 — External sandbox v1 does not require live CLI e2e tests; opt-in is env-gated

**Decision**

Live CLI end-to-end tests are **not required** for v1 acceptance of
`agent_api.exec.external_sandbox.v1`.

If e2e tests are added later, they MUST be opt-in via env gating:
- enable by setting `AGENT_API_E2E_LIVE=1` (truthy),
- select binaries via `CODEX_E2E_BINARY` (Codex) and `CLAUDE_BINARY` (Claude),
- default CI lanes MUST NOT set `AGENT_API_E2E_LIVE`; only a dedicated lane may set it.

**Context**

Concrete audit CA-0002 required removing “best-effort” ambiguity from the mapping test plan and
pinning an explicit decision about whether e2e tests are required, plus the opt-in mechanism if
deferred.

**Chosen spec**

Pinned in:
- `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/seam-5-tests.md`

**Rationale**

- The repo already uses env-gated live tests to avoid flake and missing-binary failures in default
  runs (e.g., Codex CLI e2e).
- Unit/integration tests can pin argv/RPC mapping deterministically using fake binaries and pure
  help-parser seams.

**Implications**

- When/if live e2e tests are introduced, they must include early “skip with a note” behavior when
  `AGENT_API_E2E_LIVE` is unset or binaries are unavailable.

## CRD-0013 — MCP management execution returns `Ok(output)` for non-zero exit status

**Decision**

For MCP management operations (`mcp_list/get/add/remove`):

- If the upstream CLI process is spawned and an `ExitStatus` is observed, the operation MUST return
  `Ok(AgentWrapperMcpCommandOutput { status, ... })` regardless of whether `status.success() == true`.
- Execution failures (spawn/binary missing, wait/IO errors, timeout/kill cleanup, stdout/stderr capture failures) MUST be
  surfaced as `Err(AgentWrapperError::Backend { message })`.
- On `Err(AgentWrapperError::Backend { .. })`, partial stdout/stderr MUST NOT be surfaced (no output type is returned).

**Context**

Concrete audit CA-0001 required pinning the boundary between “command completed” (with an exit status, possibly non-zero)
and “command execution failed” (spawn/timeout/IO), plus clarifying whether partial output is returned on timeouts.

**Chosen spec**

Pinned in:
- `docs/specs/universal-agent-api/mcp-management-spec.md` → “Command execution semantics (pinned)”

**Rationale**

- The MCP output type explicitly includes `status: ExitStatus`; returning `Ok(output)` on non-zero exit is the only
  contract that makes `status` consistently observable and testable.
- Keeps `AgentWrapperError::Backend` reserved for true execution faults, aligning with the contract’s safe error taxonomy.

**Implications**

- Implementations must avoid wrapper helpers that convert non-zero exit statuses into errors.
- Tests can treat `Ok(output.status != 0)` as a valid, bounded “command completed” outcome.

## CRD-0014 — MCP capability advertising + target availability are pinned by CLI manifest snapshots

**Decision**

For built-in backends, MCP capability advertising and target availability gating MUST be derived from the pinned CLI
manifest snapshots:
- Codex: `cli_manifests/codex/current.json`
- Claude Code: `cli_manifests/claude_code/current.json`

If the manifest snapshot conflicts with observed upstream CLI behavior at runtime:
- the backend MUST NOT silently change its advertised capabilities, and
- the operation MUST fail with `AgentWrapperError::Backend` (backend fault).

The required remediation is to update the pinned manifest snapshots and mapping logic in a subsequent repo update.

**Context**

Concrete audit CA-0006 required choosing an authoritative availability source for Claude MCP subcommands in v1 and defining
what happens when that source conflicts with observed CLI behavior.

**Chosen spec**

Pinned in:
- `docs/specs/universal-agent-api/mcp-management-spec.md` → “Built-in backend behavior (v1, normative)” → “Target availability source of truth (pinned)”

**Rationale**

- This repo already treats CLI manifests as the diff-reviewed inventory of upstream surfaces; using them for availability
  gating makes capability advertising deterministic and testable.
- Avoids runtime probing (flake, performance, platform differences) and keeps v1 behavior stable.

**Implications**

- Backends must implement capability advertising as a pure function of (manifest snapshot, build target, backend config).
- Drift is handled via the existing manifest update workflow, not via runtime heuristics.

## CRD-0015 — MCP transport validation and argv composition are pinned in the spec

**Decision**

For MCP add transports:

- `Stdio` final argv MUST be: `argv = command + args` (concatenation), and all items MUST be trimmed and non-empty.
- `Url.url` MUST be an absolute `http`/`https` URL (trimmed, non-empty; parsing required).
- `Url.bearer_token_env_var`, when present, MUST be a trimmed, non-empty environment variable name matching:
  `^[A-Za-z_][A-Za-z0-9_]*$`.

**Context**

Concrete audit CA-0002 and CA-0009 required removing ambiguity around:
- what inputs are considered valid for `Url` transport fields, and
- how `Stdio { command, args }` constructs the final argv passed to backend CLIs.

**Chosen spec**

Pinned in:
- `docs/specs/universal-agent-api/mcp-management-spec.md` → “Transport field validation (pinned)”

**Rationale**

- Makes request validation testable and avoids accepting inputs that are likely to be interpreted differently across
  platforms/backends.
- Provides an unambiguous mapping from typed request fields to argv construction.

**Implications**

- Implementations must validate and reject invalid transport fields before spawning any backend process.
- Backend mapping code can treat `argv` as the canonical representation for `Stdio` and derive backend-specific forms from it.

## CRD-0016 — MCP integration tests are hermetic by default; live smoke is env-gated

**Decision**

MCP integration coverage MUST be deterministic and offline by default:

- Default test mode (CI + local) uses hermetic fake binaries and runs under normal `make test` / `cargo test`.
- Optional live smoke tests against real upstream binaries are permitted, but MUST be opt-in:
  - tests are marked `#[ignore]`, and
  - enabled only when `AGENT_API_MCP_LIVE=1` is set (truthy) and the targeted backend binary is configured.

**Context**

Concrete audit CA-0008 required pinning whether MCP integration tests use real binaries vs fake binaries, how they are
gated, and how “no network required” is enforced for the test suite.

**Chosen spec**

Pinned in:
- `docs/specs/universal-agent-api/mcp-management-spec.md` → “Verification policy (this repo; v1, pinned)” → “Integration coverage + gating (pinned)”

**Rationale**

- Fake binaries can deterministically assert argv + env injection + isolated-home behavior without flake or missing-binary
  failures in CI.
- Mirrors existing repo posture: live/credentialed probes are explicit opt-ins.

**Implications**

- Live tests must be written to skip with a note when `AGENT_API_MCP_LIVE` is unset or binaries are unavailable.
- Default CI lanes must not set `AGENT_API_MCP_LIVE`.
