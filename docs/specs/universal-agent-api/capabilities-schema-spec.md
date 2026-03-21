# Schema Spec — Universal Agent API Capabilities

Status: Approved  
Approved (UTC): 2026-02-21  
Date (UTC): 2026-02-16

This spec defines `AgentWrapperCapabilities` naming and stability, including standard capability
ids such as `agent_api.exec.add_dirs.v1`.

This document is normative and uses RFC 2119 keywords (MUST/SHOULD/MUST NOT).

## Agent kind naming (normative)

`AgentWrapperKind` ids MUST:

- be lowercase ASCII
- match regex: `^[a-z][a-z0-9_]*$`
- be stable identifiers, not display names

Reserved ids (v1):

- `codex`
- `claude_code`

## Capability id naming (DR-0003)

- Core capabilities:
  - Prefix: `agent_api.`
  - Examples:
    - `agent_api.run` — backend supports the core run contract
    - `agent_api.events` — backend produces `AgentWrapperEvent`s (live or buffered)
    - `agent_api.events.live` — backend supports live streaming events
    - `agent_api.exec.add_dirs.v1` — backend supports the universal add-dirs extension key
- Backend-specific capabilities:
  - Prefix: `backend.<agent_kind>.`
  - Examples:
    - `backend.codex.exec_stream`
    - `backend.claude_code.print_stream_json`

## Capability buckets (rubric; naming convention)

Capabilities remain an open-set of strings (no additional type system), but we use standardized
prefix buckets so that capability sets can be grouped mechanically in tooling and docs.

Bucket prefixes (v1 rubric):

- `agent_api.events.*` — event stream shape/fidelity (live, delta fidelity, etc.)
- `agent_api.exec.*` — execution policy (non-interactive, approval/sandbox bridging, etc.)
- `agent_api.session.*` — conversation/thread semantics (resume/fork, session handles, etc.; orthogonal to execution policy)
- `agent_api.tools.*` — tool visibility/fidelity (calls vs results vs structured metadata)
- `agent_api.artifacts.*` — file/patch/change summaries (bounded, safe artifacts)
- `agent_api.control.*` — cancel/pause semantics and best-effort levels
- `agent_api.config.*` — cross-agent config knobs (only when truly universal)
- `backend.<agent_kind>.*` — everything agent-specific or not yet universal

Notes:

- Buckets are a naming convention only; they do not imply hierarchy or inheritance.
- New universal buckets SHOULD be introduced in this spec before shipping new `agent_api.*` ids.
- Backend-specific capabilities MUST stay under `backend.<agent_kind>.*` until the capability’s
  semantics are proven cross-agent.
- `agent_api.exec.add_dirs.v1` is the exec-bucket capability id that gates the universal add-dirs
  extension key (see Standard capability ids).

## Stability

- Core `agent_api.*` capability ids are stable once shipped.
- Backend-specific capability ids are stable per backend once shipped, but may be added over time.

## Capability matrix (generated artifact)

The repository capability matrix is a generated artifact:

- Location: `docs/specs/universal-agent-api/capability-matrix.md`
- Generator: `cargo run -p xtask -- capability-matrix`

Semantics (pinned):

- The matrix lists only capability ids advertised by at least one built-in backend at generation time (it is a union of
  `AgentWrapperBackend::capabilities().ids` across built-in backends).
- Generation is evaluated against the repository's canonical built-in target profile, not the host OS running the
  generator. In v1 that profile is:
  - `codex` -> `x86_64-unknown-linux-musl`
  - `claude_code` -> `linux-x64`
- The matrix is **not** an exhaustive registry of standard `agent_api.*` capability ids.
- If a standard capability id defined in this spec is absent from the matrix, that means no built-in backend currently
  advertises it under the generator's default built-in configs for that canonical target profile (not that the id is invalid or removed).
- Config-conditional standard capabilities may therefore be absent from the matrix when safe defaults leave them off; for
  example, `agent_api.tools.mcp.add.v1` / `agent_api.tools.mcp.remove.v1` may be absent because built-in backends default
  `allow_mcp_write` to `false`.
- Runtime availability checks MUST use `AgentWrapperCapabilities.ids` from the selected backend; the matrix is a
  maintenance/overview artifact, not a runtime truth source.

## Change control and verification (normative)

This spec is the canonical registry for standard `agent_api.*` capability ids. When a new universal
capability id is introduced or promoted:

- This spec MUST be updated in the same change that introduces the capability.
- The capability matrix MUST be regenerated, and reviewers SHOULD verify the id appears in both
  this spec and the generated matrix (noting that config-conditional capabilities may be absent
  from the matrix under default generator settings).

## Required minimum capabilities (v1, normative)

Every registered backend MUST include:

- `agent_api.run`
- `agent_api.events`

Backends that provide live streaming MUST include:

- `agent_api.events.live`

## Standard capability ids (v1, normative)

This section defines stable universal capability ids and their minimum semantics.

- `agent_api.control.cancel.v1`:
  - A backend that advertises this capability MUST support explicit cancellation via
    `AgentWrapperGateway::run_control(...)` and `AgentWrapperCancelHandle::cancel()` per
    `run-protocol-spec.md`.
  - `AgentWrapperCancelHandle::cancel()` MUST be idempotent and best-effort.
  - If cancellation is requested before `AgentWrapperRunHandle.completion` resolves,
    `AgentWrapperRunHandle.completion` MUST resolve to:
    `Err(AgentWrapperError::Backend { message: "cancelled" })`.
  - A backend that does not support explicit cancellation MUST NOT advertise this capability.
- `agent_api.tools.structured.v1`:
  - A backend that advertises this capability MUST attach `AgentWrapperEvent.data` with
    `schema="agent_api.tools.structured.v1"` on every `ToolCall` and `ToolResult` event it emits
    (per `event-envelope-schema-spec.md`).
  - A backend that does not do this MUST NOT advertise the capability.
- `agent_api.tools.results.v1`:
  - The backend can emit `ToolResult` events for tool completions and tool failures only when
    deterministically attributable (not “every failure becomes ToolResult”).
- `agent_api.tools.mcp.list.v1`:
  - The backend supports listing configured MCP servers via the non-run MCP management API (see
    `docs/specs/universal-agent-api/mcp-management-spec.md`).
- `agent_api.tools.mcp.get.v1`:
  - The backend supports retrieving a specific configured MCP server entry by name via the non-run
    MCP management API (see `docs/specs/universal-agent-api/mcp-management-spec.md`).
- `agent_api.tools.mcp.add.v1`:
  - The backend supports adding/configuring an MCP server entry via the non-run MCP management API
    (see `docs/specs/universal-agent-api/mcp-management-spec.md`).
  - This capability MUST NOT be advertised unless write enablement is explicitly configured (see
    `docs/specs/universal-agent-api/mcp-management-spec.md`).
    - For built-in backends, this means the public config field
      `agent_api::backends::codex::CodexBackendConfig.allow_mcp_write` or
      `agent_api::backends::claude_code::ClaudeCodeBackendConfig.allow_mcp_write` is `true` (both
      default to `false`), and the pinned CLI manifest snapshot shows the required subcommand is
      available on the current target.
- `agent_api.tools.mcp.remove.v1`:
  - The backend supports removing an MCP server entry by name via the non-run MCP management API
    (see `docs/specs/universal-agent-api/mcp-management-spec.md`).
  - This capability MUST NOT be advertised unless write enablement is explicitly configured (see
    `docs/specs/universal-agent-api/mcp-management-spec.md`).
    - For built-in backends, this means the public config field
      `agent_api::backends::codex::CodexBackendConfig.allow_mcp_write` or
      `agent_api::backends::claude_code::ClaudeCodeBackendConfig.allow_mcp_write` is `true` (both
      default to `false`), and the pinned CLI manifest snapshot shows the required subcommand is
      available on the current target.
- `agent_api.artifacts.final_text.v1`:
  - The backend can deterministically populate `AgentWrapperCompletion.final_text` when full
    assistant message text blocks are observed in the supported flow; `final_text=None` is valid
    otherwise.
- `agent_api.session.handle.v1`:
  - When a backend advertises this capability, it MUST surface the current run’s backend-defined
    session/thread identifier as a bounded JSON facet in:
    - exactly one early `AgentWrapperEventKind::Status` event `data` payload, and
    - `AgentWrapperCompletion.data` whenever a completion is produced and the id is known,
    per `event-envelope-schema-spec.md` ("Session handle facet (handle.v1)").
  - A backend that does not implement this MUST NOT advertise the capability.
- `agent_api.config.model.v1`:
  - Bucket: `agent_api.config.*`.
  - This is the stable capability id and R0 gate for the universal model-selection extension key.
  - Schema, trimming behavior, absence semantics, runtime-rejection posture, and backend mapping
    requirements are owned by `docs/specs/universal-agent-api/extensions-spec.md`.
  - For this capability, "deterministically honor the owner-doc semantics for a run flow" means
    the flow has exactly one pinned outcome after R0 gating and pre-spawn validation:
    either apply the accepted effective trimmed model id unchanged to the backend transport for that
    flow, or take a pinned backend-owned safe rejection path for that flow. A flow that silently
    drops, rewrites, or conditionally ignores an accepted model id does not satisfy this
    requirement.
  - A backend that advertises this capability MUST accept the same string key in
    `AgentWrapperRunRequest.extensions`, apply the owner-doc v1 semantics unchanged, and advertise
    the id only when it can deterministically honor those semantics for the targeted run flow.
  - `AgentWrapperCapabilities.ids` is a backend-global capability set, not a per-request or
    per-flow response. A backend that exposes multiple run flows MAY advertise this capability
    globally only when every exposed flow has one of the pinned deterministic outcomes above.
  - A backend that cannot deterministically honor the owner-doc semantics for one of its exposed
    run flows MUST either stop advertising this capability entirely or narrow its exposed flow set;
    it MUST NOT keep advertising while leaving that flow ambiguous.
  - Built-in backend posture for v1:
    - `codex` MAY advertise globally once exec/resume apply `--model <trimmed-id>` and fork keeps
      the pinned pre-handle safe rejection path from
      `docs/specs/codex-app-server-jsonrpc-contract.md`.
  - `claude_code` MAY advertise globally once its print exec/resume/fork flows all emit exactly
    one `--model <trimmed-id>` pair per
    `docs/specs/claude-code-session-mapping-contract.md`.
- `agent_api.exec.add_dirs.v1`:
  - Bucket: `agent_api.exec.*`.
  - This is the stable capability id and R0 gate for the universal add-dirs extension key.
  - Schema, trimming behavior, absence semantics, runtime-rejection posture, and backend mapping
    requirements are owned by `docs/specs/universal-agent-api/extensions-spec.md`.
  - A backend that advertises this capability MUST accept the same string key in
    `AgentWrapperRunRequest.extensions`, apply the owner-doc v1 semantics unchanged for every
    exposed run flow, and either map the accepted normalized directory set into backend transport
    or take a pinned safe backend-rejection path (it MUST NOT silently ignore accepted add-dir
    inputs).
  - `AgentWrapperCapabilities.ids` is a backend-global capability set, not a per-request or
    per-flow response. A backend that cannot deterministically honor the owner-doc semantics for
    one of its exposed run flows MUST either stop advertising this capability entirely or narrow
    its exposed flow set.

## Extension keys (v1, normative)

- Every supported `AgentWrapperRunRequest.extensions` key MUST be present in `AgentWrapperCapabilities.ids` as the same string.
- Core extension keys under `agent_api.*` (schema + defaults) are defined in:
  - `docs/specs/universal-agent-api/extensions-spec.md`
