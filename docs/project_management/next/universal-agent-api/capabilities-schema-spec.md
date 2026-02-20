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

## Stability

- Core `agent_api.*` capability ids are stable once shipped.
- Backend-specific capability ids are stable per backend once shipped, but may be added over time.

## Required minimum capabilities (v1, normative)

Every registered backend MUST include:

- `agent_api.run`
- `agent_api.events`

Backends that provide live streaming MUST include:

- `agent_api.events.live`

## Extension keys (v1, normative)

- Every supported `AgentWrapperRunRequest.extensions` key MUST be present in `AgentWrapperCapabilities.ids` as the same string.
- Core extension keys under `agent_api.*` (schema + defaults) are defined in:
  - `docs/project_management/next/universal-agent-api/extensions-spec.md`
