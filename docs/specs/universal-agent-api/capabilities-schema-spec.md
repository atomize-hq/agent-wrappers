# Schema Spec — Universal Agent API Capabilities

Status: Approved  
Approved (UTC): 2026-02-21  
Date (UTC): 2026-02-16

This spec defines `AgentWrapperCapabilities` naming and stability.

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
- The matrix is **not** an exhaustive registry of standard `agent_api.*` capability ids.
- If a standard capability id defined in this spec is absent from the matrix, that means no built-in backend currently
  advertises it (not that the id is invalid or removed).
- Runtime availability checks MUST use `AgentWrapperCapabilities.ids` from the selected backend; the matrix is a
  maintenance/overview artifact, not a runtime truth source.

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
    - For built-in backends, the approved v1 contract does not define a host-facing
      `allow_mcp_write` config field (see `docs/specs/universal-agent-api/contract.md`).
- `agent_api.tools.mcp.remove.v1`:
  - The backend supports removing an MCP server entry by name via the non-run MCP management API
    (see `docs/specs/universal-agent-api/mcp-management-spec.md`).
  - This capability MUST NOT be advertised unless write enablement is explicitly configured (see
    `docs/specs/universal-agent-api/mcp-management-spec.md`).
    - For built-in backends, the approved v1 contract does not define a host-facing
      `allow_mcp_write` config field (see `docs/specs/universal-agent-api/contract.md`).
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

## Extension keys (v1, normative)

- Every supported `AgentWrapperRunRequest.extensions` key MUST be present in `AgentWrapperCapabilities.ids` as the same string.
- Core extension keys under `agent_api.*` (schema + defaults) are defined in:
  - `docs/specs/universal-agent-api/extensions-spec.md`
