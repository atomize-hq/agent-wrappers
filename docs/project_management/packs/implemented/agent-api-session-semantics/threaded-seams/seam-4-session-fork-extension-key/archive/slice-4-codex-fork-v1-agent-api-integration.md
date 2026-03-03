### S4 — Codex `agent_api.session.fork.v1` integration (app-server flow + bounded events + cancellation)

- **User/system value**: Orchestrators can fork Codex sessions deterministically and headlessly using `fork.v1`, with pinned selection behavior, non-interactive safety (no approval hangs), bounded event mapping, and explicit cancellation wired to JSON-RPC `$ /cancelRequest`.
- **Scope (in/out)**:
  - In:
    - Add `agent_api.session.fork.v1` to the Codex backend:
      - `supported_extension_keys()` allowlist (fail-closed gate),
      - `AgentWrapperCapabilities.ids` only once behavior + tests land.
    - Parse and validate `extensions["agent_api.session.fork.v1"]` via the shared helper from `S1` (closed schema).
    - Enforce resume↔fork mutual exclusivity (per `extensions-spec.md`) only when both keys are supported (R0 precedence is harness-owned).
    - Implement the fork flow via `codex app-server` JSON-RPC (SA-C06; `S3`), per `docs/specs/codex-app-server-jsonrpc-contract.md`:
      - selector `"last"`:
        - compute the effective working directory (`contract.md`),
        - resolve the fork source via `thread/list` + pinned paging + deterministic selection,
        - empty scope → selection-failure pinned error (`"no session found"`).
      - selector `"id"`:
        - treat `id` as the source thread id (validated pre-spawn; whitespace-only fails).
        - unknown id → selection-failure pinned error (`"session not found"`), translated safely.
      - fork via `thread/fork` and send the prompt via `turn/start` on the forked thread.
    - Map app-server notifications into Universal Agent API events (pinned minimum, no raw payloads in `data`):
      - `agentMessage/delta` and `reasoning/text/delta` → `TextOutput`,
      - `item/started` → `ToolCall` (metadata-only),
      - `item/completed` → `ToolResult` (metadata-only),
      - `turn/started` and `turn/completed` → `Status`,
      - `error` → `Error` (bounded message, no raw details in `data`).
    - Non-interactive safety (`agent_api.exec.non_interactive`, default `true`):
      - set `approvalPolicy="never"` on both `thread/fork` and `turn/start`,
      - detect approval requests (`codex/event` with type `approval_required`/`approval`) and fail-fast with pinned `"approval required"`,
      - send `$ /cancelRequest` for the in-flight `turn/start` id before failing (best-effort).
    - Explicit cancellation:
      - implement `run_control(...).cancel()` by sending `$ /cancelRequest` for the in-flight `turn/start` id,
      - enforce pinned cancellation precedence + completion semantics from `run-protocol-spec.md`.
    - Tests:
      - agent_api integration tests for selector `"last"`/`"id"` fork flows using a fake JSON-RPC app-server,
      - pinned selection failures + terminal `Error` event rule when the stream exists,
      - approval required fail-fast behavior + `$ /cancelRequest`,
      - explicit cancellation precedence (`"cancelled"`).
  - Out:
    - Universal session listing surfaces (only internal `thread/list` usage for `"last"` selection).
    - Any new public `agent_api` Rust type shapes beyond existing specs.
- **Acceptance criteria**:
  - When Codex advertises `agent_api.session.fork.v1`:
    - selector `"last"` and `"id"` both fork and run the prompt deterministically,
    - invalid schemas fail pre-spawn with `InvalidRequest`,
    - selection failures map to pinned safe messages and obey the terminal `Error` event rule when a stream exists,
    - non-interactive mode never hangs on approval prompts and fails fast with pinned `"approval required"`,
    - explicit cancellation sends `$ /cancelRequest` and yields pinned `"cancelled"` completion outcome.
- **Dependencies**:
  - `S1` shared fork selector parser.
  - `S3` SA-C06 typed Codex app-server RPC support.
  - Normative: `docs/specs/universal-agent-api/extensions-spec.md` (schema + selection failures + R0 precedence + contradiction rules).
  - Normative: `docs/specs/codex-app-server-jsonrpc-contract.md` (RPC shapes + selection algorithm + non-interactive fail-fast + notification mapping).
  - Normative: `docs/specs/universal-agent-api/run-protocol-spec.md` (validation timing + cancellation semantics/precedence).
  - Normative: `docs/specs/universal-agent-api/event-envelope-schema-spec.md` (bounds + redaction rules).
- **Verification**:
  - `cargo test -p agent_api --features codex`
- **Rollout/safety**:
  - Capability-gated: do not advertise `agent_api.session.fork.v1` until mapping + safety + tests pass.

#### S4.T1 — Codex backend: parse/validate `fork.v1` and extract fork policy (pre-spawn)

- **Outcome**: Codex backend policy extraction supports an optional fork selector and enforces `.v1` closed-schema validation pre-spawn.
- **Inputs/outputs**:
  - Inputs: `AgentWrapperRunRequest.extensions`.
  - Outputs:
    - Extended Codex policy including `fork: Option<...>` (typed selector),
    - `InvalidRequest` for schema violations,
    - resume↔fork contradiction check wired for when both keys are supported (R0 precedence respected).
  - Files (preferred for isolation):
    - `crates/agent_api/src/backends/codex.rs` (thread policy through backend harness)
    - `crates/agent_api/src/backends/codex/fork.rs` (new; isolate fork policy + app-server wiring)
- **Implementation notes**:
  - Use the shared parser from `S1` (no duplicated JSON parsing).
  - Keep the fork codepath isolated from the existing `codex exec` streaming mapping to reduce churn.

Checklist:
- Implement:
  - Add `EXT_SESSION_FORK_V1` constant and parse it into Codex policy when present.
  - Wire fork support into `supported_extension_keys()` allowlist once behavior is implemented.

#### S4.T2 — Implement Codex fork flow via app-server: select source → fork → `turn/start`

- **Outcome**: The Codex backend can fork from `"last"` or `"id"` using SA-C06 and stream events from the follow-up `turn/start`.
- **Inputs/outputs**:
  - Inputs: typed fork selector; `AgentWrapperRunRequest.prompt`; effective working dir; non-interactive policy.
  - Outputs:
    - `AgentWrapperRunHandle` whose `events` stream reflects mapped notifications from `turn/start`,
    - completion resolves to `Ok(ExitStatus)` on success or a safe `AgentWrapperError` on failure.
  - Files:
    - `crates/agent_api/src/backends/codex/fork.rs`
- **Implementation notes**:
  - selector `"last"`:
    - compute effective working directory (`contract.md`) and pass it into `thread/list` filtering,
    - apply the pinned selection algorithm (use `S3` helper if provided).
  - selector `"id"`: use the validated id as `threadId`.
  - Call order (pinned):
    1) `initialize` (with experimentalApi)
    2) `thread/list` (only for `"last"`)
    3) `thread/fork`
    4) `turn/start`

Checklist:
- Implement:
  - Add an async task that drives the JSON-RPC sequence and forwards mapped notifications into the consumer-visible event channel.
  - Ensure selection failure short-circuits before `thread/fork` / `turn/start`.

#### S4.T3 — Implement bounded notification → `AgentWrapperEvent` mapping (no raw payloads in `data`)

- **Outcome**: App-server notifications are surfaced to callers as bounded Universal Agent API events without leaking raw backend payloads.
- **Inputs/outputs**:
  - Inputs: JSON-RPC notification method + params.
  - Outputs: `AgentWrapperEvent` stream items:
    - `TextOutput` for deltas,
    - `ToolCall`/`ToolResult` for item lifecycle (metadata-only),
    - `Status` for turn lifecycle,
    - `Error` for errors.
  - Files:
    - `crates/agent_api/src/backends/codex/fork.rs` (or a new `codex/app_server_mapping.rs` module)
- **Implementation notes**:
  - Enforce text bounds per `event-envelope-schema-spec.md` (split if needed).
  - Do not forward raw params blobs in `data`; only include safe metadata fields when necessary.

Checklist:
- Implement:
  - Add a mapping function that matches the pinned method set from `codex-app-server-jsonrpc-contract.md`.
- Validate:
  - `AgentWrapperEvent.data` is `None` (or bounded metadata-only) for app-server-derived events.

#### S4.T4 — Non-interactive safety + approval required fail-fast (`"approval required"`)

- **Outcome**: The Codex fork flow never hangs in unattended mode; approval requests trigger a deterministic, safe failure.
- **Inputs/outputs**:
  - Inputs: `agent_api.exec.non_interactive` (default `true`), JSON-RPC notifications.
  - Outputs:
    - If approval request observed:
      - send `$ /cancelRequest` for the in-flight `turn/start` id (best-effort),
      - emit exactly one terminal `Error` event with `message == "approval required"` if the stream exists,
      - resolve completion with `Err(AgentWrapperError::Backend { message: "approval required" })`.
  - Files:
    - `crates/agent_api/src/backends/codex/fork.rs`
- **Dependencies**:
  - Normative: `docs/specs/codex-app-server-jsonrpc-contract.md` (approval request definition + fail-fast ordering + cancellation).

Checklist:
- Implement:
  - Set `approvalPolicy="never"` on both `thread/fork` and `turn/start` when non-interactive is true.
  - Detect approval requests:
    - `method == "codex/event"` and payload `type` is `"approval_required"` or `"approval"` (directly or under `params.msg`).
  - On detection, cancel `turn/start` and fail fast with the pinned message.

#### S4.T5 — Selection-failure translation (pinned messages + terminal `Error` event rule)

- **Outcome**: Fork selection failures are translated into pinned safe Universal Agent API errors.
- **Inputs/outputs**:
  - Inputs: selector `"last"`/`"id"`, thread/list result, app-server errors.
  - Outputs:
    - `"no session found"` (empty `"last"` scope),
    - `"session not found"` (unknown id),
    - terminal `Error` event emission when a stream exists (per `extensions-spec.md`).
  - Files:
    - `crates/agent_api/src/backends/codex/fork.rs`
- **Implementation notes**:
  - `"last"` empty scope can be determined deterministically from `thread/list` (post-spawn but before `turn/start`).
  - `"id"` unknown/unforkable may surface as a JSON-RPC error from `thread/fork`; translate safely without embedding raw backend details.

Checklist:
- Implement:
  - Translate the two selection-failure cases into pinned `AgentWrapperError::Backend` messages.
  - Emit exactly one terminal `Error` event when the consumer-visible stream exists.

#### S4.T6 — Pin `agent_api` integration tests using a fake app-server JSON-RPC binary

- **Outcome**: Deterministic `agent_api` tests that pin Codex fork behavior without requiring a real Codex CLI.
- **Inputs/outputs**:
  - Inputs: a cross-platform fake `codex` binary that implements the fork subset of `codex app-server` JSON-RPC.
  - Outputs: new integration tests covering success + failure + safety paths.
  - Files (prefer new test file for conflict avoidance):
    - `crates/agent_api/src/bin/fake_codex_app_server_jsonrpc_agent_api.rs` (new)
    - `crates/agent_api/tests/session_fork_v1_codex.rs` (new)
- **Pinned test cases**:
  - selector `"id"`:
    - does not call `thread/list`,
    - calls `thread/fork` with `threadId == <id>`,
    - calls `turn/start` with `threadId == <forked id>` and pinned `input[]` mapping.
  - selector `"last"`:
    - calls `thread/list` with `cwd == <effective working dir>` and uses paging,
    - selects the correct source thread id via max tuple `(updatedAt, createdAt, id)`,
    - calls `thread/fork` with that id.
  - selection failures:
    - empty list → `"no session found"`,
    - unknown id → `"session not found"`,
    - terminal `Error` event rule holds when a stream exists.
  - non-interactive approval fail-fast:
    - fake server emits `codex/event` approval_required during `turn/start`,
    - backend sends `$ /cancelRequest` and fails with `"approval required"`.
  - explicit cancellation:
    - calling cancel sends `$ /cancelRequest` for the in-flight `turn/start` id,
    - completion resolves to `Err(Backend("cancelled"))` and event stream finality matches `run-protocol-spec.md`.
- **Verification**:
  - `cargo test -p agent_api --features codex`

Checklist:
- Implement:
  - Add the fake app-server JSON-RPC binary:
    - scenario-driven (env var) responses for list/fork/start/approval,
    - strict request-shape assertions (fail loudly if contract is violated),
    - `$ /cancelRequest` handling returning JSON-RPC cancelled error `code=-32800`.
  - Add integration tests for each pinned scenario.

#### S4.T7 — Advertise `agent_api.session.fork.v1` capability id (Codex)

- **Outcome**: Codex backend capabilities include `"agent_api.session.fork.v1"` only after conformance is implemented and tested.
- **Inputs/outputs**:
  - Output: Codex `capabilities().ids` includes `"agent_api.session.fork.v1"`.
  - Files:
    - `crates/agent_api/src/backends/codex.rs`

Checklist:
- Implement:
  - Add the capability id after `S4.T1`–`S4.T6` land.
