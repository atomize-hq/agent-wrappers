# Contract — Agent API Codex `stream_exec` Parity (authoritative for this feature)

Status: Draft  
Date (UTC): 2026-02-20  
Feature directory: `docs/project_management/packs/active/agent-api-codex-stream-exec/`

This document is the authoritative contract for **behavioral semantics** of the `agent_api` Codex
backend after adopting `codex::CodexClient::stream_exec` (typed event stream + completion).

Normative language: RFC 2119 requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Baselines (referenced; not duplicated)

- Universal Agent API (authoritative):
  - `docs/project_management/next/universal-agent-api/contract.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`
- Codex typed event semantics + normalization (authoritative for `ThreadEvent`):
  - `docs/specs/codex-thread-event-jsonl-parser-contract.md`
- Safety posture reference (raw line retention off-by-default):
  - `docs/adr/0007-wrapper-events-ingestion-contract.md`
  - `docs/specs/wrapper-events-ingestion-contract.md`
- Feature-local decisions (authoritative within this feature pack):
  - `docs/project_management/packs/active/agent-api-codex-stream-exec/decision_register.md`

If this contract conflicts with any baseline, the baseline wins **except** where this contract
adds stricter Codex-backend-specific requirements.

## Scope

This contract pins:

- How the Codex backend spawns/streams via `codex::CodexClient::stream_exec`.
- How `AgentWrapperRunRequest.{working_dir,timeout,env,extensions}` are applied for the Codex backend.
- Redaction rules (especially for `codex::ExecStreamError` which embeds raw JSONL lines).
- The stable mapping from Codex `ThreadEvent` to `agent_api::AgentWrapperEvent`.
- Completion finality requirements (DR-0012 preservation).
- Whether `AgentWrapperCompletion.final_text` is populated for Codex.

Out of scope:

- Changing the Universal Agent API public surface (types/fields) beyond what baselines already define.
- Opt-in “raw backend line capture” in the universal envelope (forbidden in v1 by baseline).
- Any `AgentWrapperRunRequest.extensions` keys beyond those explicitly pinned in this contract.

## Backend identity + capabilities (normative)

- `AgentWrapperKind` MUST be `"codex"`.
- `AgentWrapperCapabilities.ids` MUST include:
  - `agent_api.run`
  - `agent_api.events`
  - `agent_api.events.live`
  - `backend.codex.exec_stream`
  - `backend.codex.exec.sandbox_mode`
  - `backend.codex.exec.approval_policy`
- `AgentWrapperCapabilities.ids` MUST include `agent_api.exec.non_interactive` as defined by:
  - `docs/project_management/next/universal-agent-api/extensions-spec.md`

## Safety / approvals policy (normative)

The Codex backend is a library backend and MUST be automation-safe by default:

- It MUST disable Codex git repo checks by passing `--skip-git-repo-check` to the Codex CLI.
- It MUST NOT use “dangerously bypass approvals and sandbox” / yolo modes.
- It MUST be non-interactive by default (no approval prompts):
  - when `agent_api.exec.non_interactive` is absent or `true`, the backend MUST pass
    `--ask-for-approval never`.
- It MUST apply a deterministic sandbox mode by default:
  - when `backend.codex.exec.sandbox_mode` is absent, the backend MUST pass
    `--sandbox workspace-write`.

This policy is pinned by DR-0009 in:
- `docs/project_management/packs/active/agent-api-codex-stream-exec/decision_register.md`

## Exec policy extensions (normative)

This feature adopts the universal exec-policy core key and defines the backend-specific Codex
exec-policy knobs in a way that remains orthogonal as additional CLI agents are onboarded.

### Supported keys + types + defaults

- `agent_api.exec.non_interactive`:
  - Defined by the universal extensions registry:
    - `docs/project_management/next/universal-agent-api/extensions-spec.md`
  - This backend MUST support and honor it (and MUST advertise it in capabilities).
- `backend.codex.exec.sandbox_mode` (string enum):
  - allowed values: `read-only` | `workspace-write` | `danger-full-access`
  - default: `workspace-write`
  - meaning: the backend MUST pass `--sandbox <value>` to the Codex CLI/wrapper for that run.
  - note: `danger-full-access` is the “no sandbox” mode (hosts like Substrate may select this).
- `backend.codex.exec.approval_policy` (string enum):
  - allowed values: `untrusted` | `on-failure` | `on-request` | `never`
  - default: absent (inherit Codex CLI default) when interactive is allowed
  - meaning:
    - when present and interactive is allowed, the backend MUST pass
      `--ask-for-approval <value>`.
    - when `agent_api.exec.non_interactive=true`, the backend MUST force `never` and MUST reject
      contradictory values.

### Validation and fail-closed rules

- Unknown extension keys MUST fail-closed before spawn as `AgentWrapperError::UnsupportedCapability`.
- All supported keys MUST have their value types validated before spawn:
  - `agent_api.exec.non_interactive` MUST be boolean (per `extensions-spec.md`).
  - Codex enum values MUST match one of the allowed strings.
- Contradiction rule (normative):
  - if `agent_api.exec.non_interactive=true`, then `backend.codex.exec.approval_policy` MUST be
    absent or `"never"`; otherwise the backend MUST fail before spawn with
    `AgentWrapperError::InvalidRequest`.

## Request validation (normative)

Before spawning any backend process, the Codex backend MUST validate:

- `request.prompt.trim()` is non-empty; otherwise return `AgentWrapperError::InvalidRequest`.
- `request.extensions` contains only supported keys pinned by this contract; unknown keys MUST
  fail-closed with `AgentWrapperError::UnsupportedCapability` (baseline requirement).
- All supported extension values MUST be validated and defaults applied (see “Exec policy extensions”).

## Working directory derivation (normative)

The Codex backend MUST derive the spawned process working directory as:

1. If `request.working_dir.is_some()`: use it.
2. Else if `config.default_working_dir.is_some()`: use it.
3. Else: use `std::env::current_dir()` captured at run start.

If `std::env::current_dir()` fails, the backend MUST fail the run before spawning with
`AgentWrapperError::Backend` and a redacted message.

Codex wrapper mapping (normative; removes ambiguity):
- The backend MUST pass the derived directory to the Codex wrapper via
  `codex::CodexClientBuilder::working_dir(<dir>)` (so the spawned process uses that directory as
  its cwd).
- The backend MUST NOT use `codex::CodexClientBuilder::cd(...)` in v1 for `AgentWrapperRunRequest.working_dir`
  mapping (the universal contract for this feature defines working_dir as the spawned process cwd).

## Timeout semantics (normative)

The Codex backend MUST implement `AgentWrapperRunRequest.timeout` using the Codex wrapper timeout:

- Let `effective_timeout = request.timeout.or(config.default_timeout)`.
- If `effective_timeout.is_some()`:
  - the backend MUST enforce that timeout as an overall run timeout.
- If `effective_timeout.is_none()`:
  - the backend MUST disable the Codex wrapper’s default timeout (i.e., MUST NOT silently fall back
    to the Codex wrapper’s internal “120s default”).

Timeout failures MUST surface as `AgentWrapperError::Backend` with a **redacted** message that does
not include raw backend stdout/stderr or JSONL lines.

Codex wrapper mapping (normative; removes ambiguity):
- If `effective_timeout.is_some()`:
  - the backend MUST set `codex::CodexClientBuilder::timeout(effective_timeout.unwrap())`.
- If `effective_timeout.is_none()`:
  - the backend MUST set `codex::CodexClientBuilder::timeout(std::time::Duration::ZERO)` to disable
    the wrapper’s default timeout behavior.

## Environment semantics (normative)

### Precedence and isolation

The backend MUST apply environment variables to the spawned Codex process with these invariants:

- Request precedence: keys in `request.env` MUST override keys in `config.env`.
- No parent mutation: the backend MUST NOT mutate the parent process environment (i.e., MUST NOT
  call `std::env::set_var`, `std::env::remove_var`, or equivalent).
- No cross-run leakage: per-run env overrides MUST apply only to the spawned process of that run.

### Effective env algorithm (normative)

Let `merged_env` be:

1. Start with `config.env`.
2. For each `(k, v)` in `request.env`, set/override `merged_env[k] = v`.

The backend MUST ensure `merged_env` is applied to the spawned Codex process **after** Codex wrapper
internal environment injection (e.g., `CODEX_HOME`, `CODEX_BINARY`, default `RUST_LOG`) so that:

- if `merged_env` contains `CODEX_HOME`, it overrides any `config.codex_home` injection.
- if `merged_env` contains `RUST_LOG`, it overrides any wrapper-provided default.

### Required Codex wrapper API (normative; C0 dependency)

To satisfy per-run env semantics while still executing via typed streaming, the Codex backend MUST
use the additive Codex wrapper API defined by `C0-spec.md`:

- `codex::CodexClient::stream_exec_with_env_overrides(exec_request, &merged_env)`

## Redaction policy (normative)

### Absolute prohibition

The Codex backend MUST NOT emit raw JSONL lines anywhere in:

- `AgentWrapperEvent.text`
- `AgentWrapperEvent.message`
- `AgentWrapperEvent.data`
- `AgentWrapperCompletion.final_text`
- `AgentWrapperCompletion.data`
- `AgentWrapperError::{InvalidRequest,Backend,UnsupportedCapability,...}.message`

In particular:

- The backend MUST NOT use `codex::ExecStreamError::to_string()` / `Display` as an emitted message.
- The backend MUST NOT copy `ExecStreamError::{Parse,Normalize}.line` into any emitted field.

### Safe error mapping requirement

All Codex-wrapper errors MUST be mapped to **redacted summaries** before emission. The authoritative
mapping algorithm is defined in:

- `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md`

## Event mapping (normative)

The Codex backend MUST map Codex typed events (`codex::ThreadEvent`) to the universal envelope as
follows (stable v1 contract):

- `ThreadStarted`, `TurnStarted`, `TurnCompleted` → `AgentWrapperEventKind::Status`
  - `channel = Some("status")`
  - `message = None` (best-effort status message is allowed but not required)
- `TurnFailed` → `AgentWrapperEventKind::Status`
  - `channel = Some("status")`
  - `message = Some("turn failed")`
- `Error`, `item_type=error` → `AgentWrapperEventKind::Error`
  - `channel = Some("error")`
  - `message = Some(<safe, bounded>)`
- `ItemFailed` → `AgentWrapperEventKind::Error` (default)
  - `channel = Some("error")`
  - `message = Some(<safe, bounded>)`
  - Emit `ToolResult(phase="fail", status="failed")` for `ThreadEvent::ItemFailed` **only when** `item.extra["item_type"]` exists, **is a string**, and is in `{ "command_execution", "file_change", "mcp_tool_call", "web_search" }`. Otherwise, keep `ItemFailed → Error`.
- `item_type=agent_message|reasoning` → `AgentWrapperEventKind::TextOutput`
  - `channel = Some("assistant")`
  - `text = Some(<chunk>)` (snapshot text for started/completed; delta text for deltas)
- `item_type=command_execution|file_change|mcp_tool_call|web_search`:
  - `item.started`, `item.delta` → `AgentWrapperEventKind::ToolCall`
    - `channel = Some("tool")`
  - `item.completed` → `AgentWrapperEventKind::ToolResult`
    - `channel = Some("tool")`
- `item_type=todo_list` → `AgentWrapperEventKind::Status`
  - `channel = Some("status")`

For all mapped events, the backend MUST obey the stable payload rules and bounds enforcement in the
universal baselines (`event-envelope-schema-spec.md` and `contract.md`).

## Completion mapping + finality (normative)

### DR-0012 preservation (completion finality)

The Codex backend MUST preserve DR-0012 from the universal run protocol:

- `AgentWrapperRunHandle.completion` MUST NOT resolve until the **universal events stream is final**
  (terminated or dropped by the consumer).

### Completion payload

On successful Codex process exit (including non-zero exit codes), the backend MUST produce:

- `AgentWrapperCompletion.status = <Codex exit status>`
- `AgentWrapperCompletion.data = None` (v1)

#### `final_text` policy (normative; pinned)

The Codex backend MUST populate `AgentWrapperCompletion.final_text` using the Codex wrapper’s
completion artifact (`ExecCompletion.last_message`) with the following deterministic rule:

- `final_text = Some(s)` iff the upstream completion returns `Ok(ExecCompletion { last_message: Some(s), .. })`.
- Otherwise `final_text = None`.

Bounds enforcement (normative):

- If `final_text` exceeds `65536` bytes (UTF-8), it MUST be truncated UTF-8-safely and suffixed with
  `…(truncated)`.

#### Non-zero exit completion semantics (normative)

If the underlying Codex process exits non-zero, the backend MUST:

- return `Ok(AgentWrapperCompletion { status: <non-zero>, ... })` (completion is not an error), and
- set `AgentWrapperCompletion.final_text = None`, and
- emit a best-effort `AgentWrapperEventKind::Error` event with a **redacted** message (stderr MUST
  NOT be emitted).

Rationale: this preserves the universal “completion resolves with exit status” mental model while
maintaining a safe-by-default posture.
