# Schema Spec — Universal Agent API Capabilities

Status: Draft  
Date (UTC): 2026-02-16

This spec defines `AgentWrapperCapabilities` naming and stability.

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
- `agent_api.tools.*` — tool visibility/fidelity (calls vs results vs structured metadata)
- `agent_api.artifacts.*` — file/patch/change summaries (bounded, safe artifacts)
- `agent_api.control.*` — cancel/pause semantics and best-effort levels
- `agent_api.config.*` — cross-agent config knobs (only when truly universal)
- `agent_api.obs.*` — observability hooks (run ids, trace hooks, etc.)
- `backend.<agent_kind>.*` — everything agent-specific or not yet universal

Notes:

- Buckets are a naming convention only; they do not imply hierarchy or inheritance.
- New universal buckets SHOULD be introduced in this spec before shipping new `agent_api.*` ids.
- Backend-specific capabilities MUST stay under `backend.<agent_kind>.*` until the capability’s
  semantics are proven cross-agent.

## Stability

- Core `agent_api.*` capability ids are stable once shipped.
- Backend-specific capability ids are stable per backend once shipped, but may be added over time.

## Required minimum capabilities (v1, normative)

Every registered backend MUST include:

- `agent_api.run`
- `agent_api.events`

Backends that provide live streaming MUST include:

- `agent_api.events.live`

## Standard capability ids (v1, normative)

This section defines stable universal capability ids and their minimum semantics.

- `agent_api.tools.structured.v1`:
  - A backend that advertises this capability MUST attach `AgentWrapperEvent.data` with
    `schema="agent_api.tools.structured.v1"` on every `ToolCall` and `ToolResult` event it emits
    (per `event-envelope-schema-spec.md`).
  - A backend that does not do this MUST NOT advertise the capability.
- `agent_api.tools.results.v1`:
  - The backend can emit `ToolResult` events for tool completions and tool failures only when
    deterministically attributable (not “every failure becomes ToolResult”).
- `agent_api.artifacts.final_text.v1`:
  - The backend can deterministically populate `AgentWrapperCompletion.final_text` when full
    assistant message text blocks are observed in the supported flow; `final_text=None` is valid
    otherwise.

## Extension keys (v1, normative)

- Every supported `AgentWrapperRunRequest.extensions` key MUST be present in `AgentWrapperCapabilities.ids` as the same string.
- Core extension keys under `agent_api.*` (schema + defaults) are defined in:
  - `docs/project_management/next/universal-agent-api/extensions-spec.md`
