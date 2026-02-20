# Protocol Spec — Codex `ExecStream` → `AgentWrapperRunHandle` Adapter

Status: Draft  
Date (UTC): 2026-02-20  
Feature directory: `docs/project_management/packs/active/agent-api-codex-stream-exec/`

This spec defines the **execution protocol** used by the `agent_api` Codex backend when adapting
`codex::CodexClient::stream_exec` (typed event stream + completion) into the universal run handle.

Normative language: RFC 2119 requirement keywords (`MUST`, `MUST NOT`, `SHOULD`).

## Baselines (referenced; not duplicated)

- Universal run finality semantics (DR-0012):
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/decision_register.md` (DR-0012)
- Universal envelope bounds + raw-line prohibition:
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`
- Codex typed event semantics:
  - `docs/specs/codex-thread-event-jsonl-parser-contract.md`
- Ingestion safety posture reference:
  - `docs/specs/wrapper-events-ingestion-contract.md`

## Definitions

- **Upstream handle**: `codex::ExecStream { events, completion }`.
  - `events`: `Stream<Item = Result<codex::ThreadEvent, codex::ExecStreamError>>`
  - `completion`: `Future<Output = Result<codex::ExecCompletion, codex::ExecStreamError>>`
- **Downstream handle**: `agent_api::AgentWrapperRunHandle { events, completion }`.

## Adapter requirements (normative)

### R0 — Prompt and extension validation

Before spawning any Codex process, the adapter MUST:

1. Reject empty prompts (`request.prompt.trim().is_empty()`) as `AgentWrapperError::InvalidRequest`.
2. Validate `request.extensions` per the contract:
   - unknown keys MUST fail-closed as `AgentWrapperError::UnsupportedCapability`
   - known keys MUST have types validated and defaults applied
   - contradiction rules MUST be enforced (e.g., non-interactive implies approval policy `never`).

### R1 — Codex client configuration invariants

The adapter MUST configure Codex streaming so that universal safety invariants are not violated:

- JSON mode MUST be enabled (`--json`), and the adapter MUST consume typed events.
- Non-interactive mode MUST be deterministic by default (DR-0009):
  - when `agent_api.exec.non_interactive` is absent or `true`, the adapter MUST configure the
    Codex wrapper to pass `--ask-for-approval never`.
- Sandbox mode MUST be deterministic by default (DR-0009):
  - when `backend.codex.exec.sandbox_mode` is absent, the adapter MUST configure the Codex wrapper
    to pass `--sandbox workspace-write`.
- Git repo checks MUST be disabled:
  - the spawned CLI MUST include `--skip-git-repo-check` so runs do not depend on being inside a
    git repository.
- The adapter MUST NOT use Codex “danger” safety overrides:
  - MUST NOT pass `--dangerously-bypass-approvals-and-sandbox` (or yolo equivalents).
- Raw JSONL lines MUST NOT be mirrored to the parent stdout:
  - Codex wrapper `mirror_stdout` MUST be `false` for this backend.
- Codex stderr MUST NOT be mirrored to the parent stderr (to avoid secret leakage):
  - Codex wrapper `quiet` MUST be `true` for this backend.
- The adapter MUST NOT request raw JSONL tee-to-disk logging (no `json_event_log`) in v1.

Working directory and timeout mapping (normative; removes ambiguity):
- The adapter MUST map the derived universal working directory to the Codex wrapper by setting:
  - `codex::CodexClientBuilder::working_dir(<derived_dir>)`
- The adapter MUST NOT use `codex::CodexClientBuilder::cd(...)` for universal `working_dir` mapping in v1.
- The adapter MUST map timeouts exactly as pinned in `contract.md`:
  - if `effective_timeout.is_some()`: `CodexClientBuilder::timeout(effective_timeout.unwrap())`
  - else: `CodexClientBuilder::timeout(Duration::ZERO)`

Exec policy mapping (normative; removes ambiguity):
- If `agent_api.exec.non_interactive=true`:
  - the adapter MUST set the Codex wrapper approval policy to `never`.
- If `agent_api.exec.non_interactive=false`:
  - if `backend.codex.exec.approval_policy` is present, the adapter MUST set
    `--ask-for-approval <value>`.
  - if it is absent, the adapter MUST omit `--ask-for-approval` (inherit upstream defaults).
- The adapter MUST always set `--sandbox <mode>` using:
  - `backend.codex.exec.sandbox_mode` when present, else the default `workspace-write`.

### R2 — Env precedence and no-mutation

The adapter MUST implement env precedence/isolation exactly as defined by:

- `docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`

Additionally, the adapter MUST ensure the merged env is applied to the spawned Codex process even
though spawning is performed inside the Codex wrapper crate.

This requires a Codex wrapper API that supports per-invocation env injection while using
`CodexClient::stream_exec` (C0 deliverable).

### R3 — Live streaming and ordering

- Event ordering: the adapter MUST emit universal events in the same order as the upstream
  `ExecStream.events` yields them.
- Bounds splitting: if one upstream event maps to a universal event that exceeds bounds, the
  adapter MUST split/truncate/drop according to the baseline bounds rules, and MUST preserve
  ordering of the resulting events.

### R4 — Completion finality (DR-0012)

The adapter MUST preserve DR-0012 by ensuring:

- The downstream completion future MUST NOT resolve until the downstream events stream is final
  (terminated or dropped).

Concrete requirement:

- The adapter MUST use a “gated completion” mechanism equivalent to
  `crates/agent_api/src/run_handle_gate.rs` behavior:
  - completion waits for both:
    - an internal completion result, and
    - an “events stream done/dropped” signal.

### R5 — Downstream drop/backpressure behavior (deadlock avoidance)

If the downstream event receiver is dropped (consumer stops reading events), the adapter MUST:

- Continue draining the upstream `ExecStream.events` stream until it terminates.
- Stop forwarding mapped events after the first failed send, but MUST NOT stop polling upstream.

This prevents deadlocks on bounded channels inside the Codex wrapper (and preserves the ability for
the upstream completion to resolve).

## Redaction mapping (normative)

### Absolute prohibition

The adapter MUST NOT emit any raw JSONL line content embedded in upstream errors. Specifically:

- MUST NOT use `ExecStreamError::Display` / `to_string()` for emitted messages (it includes `` `line` ``).
- MUST NOT surface `ExecStreamError::{Parse,Normalize}.line` in any emitted field.

### Canonical redaction function (normative)

The adapter MUST implement a deterministic redaction mapping:

Let `redact_exec_stream_error(err: &ExecStreamError) -> String` be:

- `ExecStreamError::Parse { source, line }` → `"codex stream parse error (redacted): {source} (line_bytes={n})"`
- `ExecStreamError::Normalize { message, line }` → `"codex stream normalize error (redacted): {message} (line_bytes={n})"`
- `ExecStreamError::IdleTimeout { idle_for }` → `"codex stream idle timeout: {idle_for:?}"`
- `ExecStreamError::ChannelClosed` → `"codex stream closed unexpectedly"`
- `ExecStreamError::Codex(CodexError::NonZeroExit { status, .. })` → `"codex exited non-zero: {status:?} (stderr redacted)"`
- `ExecStreamError::Codex(other)` → `"codex backend error: {kind} (details redacted when unsafe)"`

Where:

- `{n}` is `line.as_bytes().len()`.
- `{kind}` is a stable category label determined only from the `CodexError` variant, and MUST be
  one of:
  - `spawn`
  - `wait`
  - `timeout`
  - `io`
  - `invalid_request`
  - `other`

All emitted universal errors/events MUST use only this redacted form (plus baseline truncation).

### `CodexError` → `{kind}` mapping (normative)

Let `codex_error_kind(err: &codex::CodexError) -> &'static str` be:

- `CodexError::Spawn { .. }` → `spawn`
- `CodexError::Wait { .. }` → `wait`
- `CodexError::Timeout { .. }` → `timeout`
- Any “empty input” invalid request:
  - `CodexError::EmptyPrompt` → `invalid_request`
  - `CodexError::EmptySandboxCommand` → `invalid_request`
  - `CodexError::EmptyExecPolicyCommand` → `invalid_request`
  - `CodexError::EmptyApiKey` → `invalid_request`
  - `CodexError::EmptyTaskId` → `invalid_request`
  - `CodexError::EmptyEnvId` → `invalid_request`
  - `CodexError::EmptyMcpServerName` → `invalid_request`
  - `CodexError::EmptyMcpCommand` → `invalid_request`
  - `CodexError::EmptyMcpUrl` → `invalid_request`
  - `CodexError::EmptySocketPath` → `invalid_request`
- Any filesystem/process I/O failure:
  - `CodexError::TempDir { .. }` → `io`
  - `CodexError::WorkingDirectory { .. }` → `io`
  - `CodexError::PrepareOutputDirectory { .. }` → `io`
  - `CodexError::PrepareCodexHome { .. }` → `io`
  - `CodexError::StdoutUnavailable` → `io`
  - `CodexError::StderrUnavailable` → `io`
  - `CodexError::StdinUnavailable` → `io`
  - `CodexError::CaptureIo { .. }` → `io`
  - `CodexError::StdinWrite { .. }` → `io`
  - `CodexError::ResponsesApiProxyInfoRead { .. }` → `io`
- Any other Codex wrapper/internal failure:
  - `CodexError::InvalidUtf8 { .. }` → `other`
  - `CodexError::JsonParse { .. }` → `other`
  - `CodexError::ExecPolicyParse { .. }` → `other`
  - `CodexError::FeatureListParse { .. }` → `other`
  - `CodexError::ResponsesApiProxyInfoParse { .. }` → `other`
  - `CodexError::Join { .. }` → `other`

Note:
- `CodexError::NonZeroExit { .. }` is handled explicitly by the `ExecStreamError` mapping above and
  MUST NOT be categorized via `{kind}`.
- Any of:
  - `TempDir`
  - `WorkingDirectory`
  - `PrepareOutputDirectory`
  - `PrepareCodexHome`
  - `StdoutUnavailable`
  - `StderrUnavailable`
  - `StdinUnavailable`
  - `CaptureIo`
  - `StdinWrite`
  - `Join`
  - `ResponsesApiProxyInfoRead`
  → `io`
- Otherwise → `other`

## Mapping rules (normative)

### ThreadEvent → AgentWrapperEvent

For each upstream `Ok(thread_event)` item, the adapter MUST map to exactly one **logical**
`AgentWrapperEvent` as defined by `contract.md` in this feature directory, and then apply the
universal bounds enforcement rules (which may split/truncate/drop fields deterministically).

### Upstream error item → AgentWrapperEvent

For each upstream `Err(exec_err)` item from `ExecStream.events`, the adapter MUST emit exactly one:

- `AgentWrapperEventKind::Error`
  - `channel = Some("error")`
  - `message = Some(redact_exec_stream_error(&exec_err))`
  - `text = None`
  - `data = None`

The adapter MUST then continue processing subsequent upstream items (per-line isolation).

## Completion mapping (normative)

When the upstream completion future resolves:

- If `Ok(exec_completion)`:
  - downstream completion MUST succeed with:
    - `status = exec_completion.status`
    - `final_text = Some(last_message)` iff `exec_completion.last_message.is_some()` (else `None`)
      - bounds: if `last_message` exceeds `65536` bytes UTF-8, it MUST be truncated UTF-8-safely
        and suffixed with `…(truncated)`
    - `data = None` (v1)
- If `Err(exec_err)`:
  - If `exec_err` is `ExecStreamError::Codex(CodexError::NonZeroExit { status, .. })`:
    - downstream completion MUST succeed with:
      - `status = status` (non-zero)
      - `final_text = None`
      - `data = None`
    - and the adapter MUST emit a best-effort `AgentWrapperEventKind::Error` with:
      - `message = "codex exited non-zero: {status:?} (stderr redacted)"`
  - Otherwise:
    - downstream completion MUST be `Err(AgentWrapperError::Backend)` with:
      - `message = redact_exec_stream_error(&exec_err)` (bounded/truncated by baseline rules)

## Reference adapter algorithm (normative, step-by-step)

An implementation conforms to this spec if it implements the following algorithm (variable names
are illustrative; semantics are normative):

1. Validate prompt and `extensions` per R0.
2. Create downstream `mpsc::channel::<AgentWrapperEvent>(32)` and a completion `oneshot`.
3. Spawn a single task that:
   1. Builds a `codex::CodexClient` from backend config + request (working dir, timeout, binary,
      codex_home), with `json=true`, `mirror_stdout=false`, `quiet=true`.
   2. Computes `merged_env` (config env + request env, request wins).
   3. Calls the Codex wrapper streaming API using `merged_env` (C0) via:
      - `codex::CodexClient::stream_exec_with_env_overrides(exec_request, &merged_env)`
   4. Drains `ExecStream.events`:
      - Maintain `forward = true`.
      - For each item:
        - Map `Ok(ThreadEvent)` to a universal event per `contract.md`.
        - Map `Err(ExecStreamError)` to one universal error event using the redaction function.
        - Enforce bounds per the universal baseline (`event-envelope-schema-spec.md`).
        - If `forward` and sending fails, set `forward=false` and continue draining upstream
          without forwarding.
   5. Await upstream completion and map to downstream completion per “Completion mapping”.
   6. If emitting a best-effort “non-zero exit” error event, do so before dropping the downstream
      sender.
   7. Drop the downstream sender, then send the completion result via the oneshot.
4. Return a downstream `AgentWrapperRunHandle` whose completion is gated to the downstream events
   finality per DR-0012 (R4).
