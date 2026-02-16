# Schema Spec — Universal Agent API Event Envelope

Status: Draft  
Date (UTC): 2026-02-16

This spec defines the stable schema/invariants for `AgentEvent`.

## Fields (minimum)

- `agent_kind` (string-backed `AgentKind`)
- `kind` (`AgentEventKind`)
- `channel` (optional string)
- `data` (optional JSON value)

## Constraints

- `channel`:
  - optional
  - bounded length: implementation MUST enforce `len(channel) <= 128` (bytes, UTF-8)
  - intended for best-effort grouping (e.g., `"tool"`, `"error"`, `"status"`)
- `data`:
  - optional
  - bounded: implementation MUST enforce `serialized_json_bytes(data) <= 65536` (64 KiB)
  - MUST NOT contain raw backend lines by default
  - MAY contain backend-specific structured payloads when safe and bounded

## Kind mapping rules

- Backends map their native event types to the stable kinds.
- If the backend cannot classify an event, it must use `Unknown`.

## Channel suggestions (non-normative)

Recommended channel values when applicable:
- `tool`
- `error`
- `status`
- `assistant`
- `user`

## Safety (normative)

- Backends MUST NOT emit raw line content from upstream processes in v1.
- If a downstream consumer needs raw lines, it MUST capture them at the ingestion boundary itself
  (outside `AgentEvent.data`), rather than expanding the universal event contract.
