# Schema Spec — Universal Agent API Event Envelope

Status: Draft  
Date (UTC): 2026-02-16

This spec defines the stable schema/invariants for `AgentWrapperEvent`.

Definition (v1):
- “raw backend lines” means unparsed stdout/stderr line capture from the spawned CLI process.

## Fields (minimum)

- `agent_kind` (string-backed `AgentWrapperKind`)
- `kind` (`AgentWrapperEventKind`)
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

## Tools facet (structured.v1) (v1, normative)

When a backend advertises capability id `agent_api.tools.structured.v1`, it MUST attach a tools
facet in `AgentWrapperEvent.data` for every event where `kind ∈ {ToolCall, ToolResult}`.

This MUST apply even when the backend would otherwise omit `data`: for `ToolCall` and `ToolResult`
events, the backend MUST set `data = Some({ "schema": "agent_api.tools.structured.v1", "tool": { ... } })`.

This requirement is subject to the existing `data` size bound and enforcement behavior: if the tools
facet would exceed the 64 KiB serialized `data` bound, the backend MUST apply the baseline oversize
replacement (`{"dropped": {"reason": "oversize"}}`).

### Schema

```json
{
  "schema": "agent_api.tools.structured.v1",
  "tool": {
    "backend_item_id": "string|null",
    "thread_id": "string|null",
    "turn_id": "string|null",

    "kind": "string",
    "phase": "start|delta|complete|fail",
    "status": "pending|running|completed|failed|unknown",
    "exit_code": "integer|null",

    "bytes": { "stdout": "integer", "stderr": "integer", "diff": "integer", "result": "integer" },

    "tool_name": "string|null",
    "tool_use_id": "string|null"
  }
}
```

### Field rules (v1, normative)

- `tool.kind` is an open set.
- `bytes.*` are integer counts; use `0` when absent/unknown.
- `exit_code` is `integer|null`.

Safety (v1, normative):
- The tools facet is metadata-only.
- `AgentWrapperEvent.data` MUST NOT include raw tool inputs/outputs, raw backend lines, diffs/patches,
  or tool payload JSON.

### Recommended `tool.kind` values (non-normative)

- Codex built-ins: `command_execution`, `file_change`, `mcp_tool_call`, `web_search`
- Claude built-ins: `tool_use`, `tool_result`

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

`AgentWrapperCompletion.data` MUST follow the same size limit and enforcement behavior as `AgentWrapperEvent.data`:

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
  (outside `AgentWrapperEvent.data`), rather than expanding the universal event contract.
