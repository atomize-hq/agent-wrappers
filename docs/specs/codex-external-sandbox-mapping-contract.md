# Codex External Sandbox Mapping Contract (v1)

Status: **Normative**  
Scope: concrete Codex backend mapping for the dangerous universal extension key
`agent_api.exec.external_sandbox.v1` across all Codex entrypoints (exec / resume / fork).

## Normative language

This document uses RFC 2119-style requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Purpose

The Universal Agent API extensions registry owns the schema, defaults, and cross-key validation for
`agent_api.exec.external_sandbox.v1`:

- `docs/specs/universal-agent-api/extensions-spec.md`

This document defines the **Codex backend-owned**, testable mapping from an accepted
`agent_api.exec.external_sandbox.v1 == true` request into:

- Codex CLI exec/resume argv, and
- Codex app-server JSON-RPC params for fork flows.

## Baselines (referenced; not duplicated)

- Key semantics + validation + required warning event:
  - `docs/specs/universal-agent-api/extensions-spec.md`
- Codex app-server JSON-RPC field names and allowed values:
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
- Codex wrapper canonical flag spelling (`--dangerously-bypass-approvals-and-sandbox`):
  - `docs/specs/codex-wrapper-coverage-scenarios-v1.md` (Scenario 0: root flags)

## Preconditions (normative)

When `agent_api.exec.external_sandbox.v1` is requested:

- The key MUST be capability-gated and validated per `extensions-spec.md` (R0 fail-closed).
- The backend MUST remain non-interactive (MUST NOT hang on prompts).
- The request MUST NOT include any `backend.<agent_kind>.exec.*` keys (invalid per `extensions-spec.md`).

This document defines mapping only for the valid case where:

- the backend advertises the capability id, and
- the request passes validation and contradiction checks.

## Exec/resume mapping (Codex CLI) (v1, pinned)

When `extensions["agent_api.exec.external_sandbox.v1"] == true` is accepted and the Codex backend
uses the Codex CLI `exec`/`resume` entrypoints, the backend MUST:

- apply the dangerous safety override by configuring the wrapper as:
  - `codex::CodexClientBuilder::dangerously_bypass_approvals_and_sandbox(true)`
- ensure the spawned argv contains exactly one `--dangerously-bypass-approvals-and-sandbox`.

Exactness (pinned):

- The spawned argv MUST NOT contain any of:
  - `--full-auto`
  - `--ask-for-approval`
  - `--sandbox`

Rationale: the dangerous bypass override collapses approval/sandbox behavior; mixing it with other
exec-policy flags creates ambiguous posture and makes tests non-deterministic.

Unavailable mapping primitive behavior (pinned):

- The backend MUST NOT attempt a fallback mapping (no spawn then retry with different flags).
- If the installed Codex binary rejects the pinned dangerous override, the backend MUST fail the
  run as `AgentWrapperError::Backend { message }` with a safe/redacted `message` (MUST NOT embed raw
  Codex output).

## Fork mapping (Codex app-server JSON-RPC) (v1, pinned)

When `extensions["agent_api.exec.external_sandbox.v1"] == true` is accepted and the Codex backend
uses `codex app-server` JSON-RPC for fork flows, the backend MUST set the following params on the
wire (field names are pinned by `codex-app-server-jsonrpc-contract.md`):

Applicable method surfaces (pinned):

- Required (fork flow): `thread/fork`, `turn/start`
- Optional (only when fork selector is `"last"`): `thread/list` (selection only; no exec-policy override fields)

- `thread/fork` request params:
  - `approvalPolicy = "never"`
  - `sandbox = "danger-full-access"`
- `turn/start` request params:
  - `approvalPolicy = "never"`

Note (pinned):

- `turn/start` does not support a `sandbox` parameter in this repo’s pinned `turn/start` v2 subset
  (`docs/specs/codex-app-server-jsonrpc-contract.md`). Therefore, the sandbox override applies only
  via `thread/fork`.

Unavailable mapping primitive behavior (pinned):

- The backend MUST NOT attempt a fallback mapping (no spawn then retry with different params).
- If the app-server rejects the pinned `approvalPolicy` / `sandbox` values (surfaced as JSON-RPC
  error responses to `thread/fork` and/or `turn/start` per `codex-app-server-jsonrpc-contract.md`),
  the backend MUST fail the run as `AgentWrapperError::Backend { message }` with a safe/redacted
  `message` (MUST NOT embed raw server output).
- If (and only if) the consumer-visible `events` stream is still open, the backend MUST emit
  exactly one terminal `AgentWrapperEventKind::Error` event (with the same safe/redacted message)
  before closing the stream (per `docs/specs/universal-agent-api/run-protocol-spec.md`, "Error event
  emission for post-spawn unsupported operations (backend fault)").
