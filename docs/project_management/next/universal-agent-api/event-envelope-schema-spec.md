# Schema Spec — Universal Agent API Event Envelope

Status: Draft  
Date (UTC): 2026-02-16

This spec defines the stable schema/invariants for `AgentEvent`.

Definition (v1):
- “raw backend lines” means unparsed stdout/stderr line capture from the spawned CLI process.

## Fields (minimum)

- `agent_kind` (string-backed `AgentKind`)
- `kind` (`AgentEventKind`)
- `channel` (optional string)
- `text` (optional string; stable for `TextOutput`)
- `message` (optional string; stable for `Status` and `Error`)
- `data` (optional JSON value)

## Constraints

- `channel`:
  - optional
  - bounded length: implementation MUST enforce `len(channel) <= 128` (bytes, UTF-8)
  - intended for best-effort grouping (e.g., `"tool"`, `"error"`, `"status"`)
- `text`:
  - bounded: implementation MUST enforce `len(text) <= 65536` (bytes, UTF-8)
  - if a backend produces text larger than the bound, it MUST split it into multiple `TextOutput`
    events (preserving order) so each event satisfies the bound
- `message`:
  - bounded: implementation MUST enforce `len(message) <= 4096` (bytes, UTF-8)
- `data`:
  - optional
  - bounded: implementation MUST enforce `serialized_json_bytes(data) <= 65536` (64 KiB)
  - MUST NOT contain raw backend lines in v1
  - MAY contain backend-specific structured payloads when safe and bounded

`serialized_json_bytes(value)` is defined as `serde_json::to_vec(value).len()`.

## Enforcement behavior (v1, normative)

- If `channel` exceeds the bound, the backend MUST set `channel=None` for that event.
- If `message` exceeds the bound, the backend MUST enforce the following algorithm (ensuring valid UTF-8):
  - Let `suffix = "…(truncated)"`.
  - If `bound_bytes > len(suffix_bytes)`:
    - truncate message to `bound_bytes - len(suffix_bytes)` bytes (UTF-8 safe) and append `suffix`.
  - Else:
    - set `message` to `"…"` truncated to `bound_bytes` bytes.
- If `data` exceeds the bound, the backend MUST replace it with:
  - `{"dropped": {"reason": "oversize"}}`

## Completion payload bounds (v1, normative)

`AgentCompletion.data` MUST follow the same size limit and enforcement behavior as `AgentEvent.data`:

- bounded: `serialized_json_bytes(data) <= 65536`
- if oversized: replace with `{"dropped": {"reason": "oversize"}}`

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
