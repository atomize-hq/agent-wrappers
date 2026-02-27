# Schema Spec — Universal Agent API Event Envelope

Status: Approved  
Approved (UTC): 2026-02-21  
Date (UTC): 2026-02-16

This spec defines the stable schema/invariants for `AgentWrapperEvent`.

This document is normative and uses RFC 2119 keywords (MUST/SHOULD/MUST NOT).

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
events, the backend MUST set `data` to a JSON object containing at minimum:
`{ "schema": "agent_api.tools.structured.v1", "tool": { ... } }`.

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
  },
  "obs": { "schema": "agent_api.obs.v1", "...": "..." }
}
```

### Field rules (v1, normative)

- `tool.kind` is an open set.
- `bytes.*` are integer counts; use `0` when absent/unknown.
- `exit_code` is `integer|null`.
- If present, `obs` MUST conform to the obs facet schema defined in this document (see "Obs facet (v1)").

Safety (v1, normative):
- The tools facet is metadata-only.
- `AgentWrapperEvent.data` MUST NOT include raw tool inputs/outputs, raw backend lines, diffs/patches,
  or tool payload JSON.

### Recommended `tool.kind` values (non-normative)

- Codex built-ins: `command_execution`, `file_change`, `mcp_tool_call`, `web_search`
- Claude built-ins: `tool_use`, `tool_result`

## Obs facet (v1) (v1, normative)

This spec defines a bounded, metadata-only **obs facet** for correlating events and completions
across ingestion systems (run ids, trace propagation, and tags).

Key constraint: `ToolCall` / `ToolResult` events already use `data.schema="agent_api.tools.structured.v1"`;
therefore obs metadata MUST NOT compete for `data.schema` on tool events. Instead, obs metadata is carried
under an optional nested `data.obs` object.

Capability gating (v1, normative):
- A backend MUST NOT emit an obs facet unless it advertises capability id `agent_api.obs.v1`.

### Placement (v1, normative)

When present:
- `AgentWrapperEvent.data` MUST be a JSON object and MAY include an `obs` key.
- `AgentWrapperCompletion.data` MUST be a JSON object and MAY include an `obs` key.

The `obs` object:
- MAY appear on any `AgentWrapperEventKind` (including `ToolCall` and `ToolResult`).
- SHOULD be stable for the entire run (i.e., the `obs` object SHOULD be identical across all events and the completion),
  except when enforcement behavior drops fields due to bounds.

Reserved key (v1, normative):
- When `data` is an object, the top-level key `obs` is reserved for the obs facet and MUST NOT be repurposed for
  backend-specific payloads.

If a backend would otherwise attach a non-object `data` payload, it SHOULD instead wrap that payload in an object
so that reserved facet keys like `obs` can coexist without schema conflicts.

### Schema (agent_api.obs.v1) (v1, normative)

`data.obs` (and `completion.data.obs`) MUST conform to:

```json
{
  "schema": "agent_api.obs.v1",
  "run_id": "string|null",
  "trace_context": {
    "traceparent": "string|null",
    "tracestate": "string|null",
    "baggage": "string|null"
  }|null,
  "tags": { "k": "v" }|null
}
```

Field meaning (v1, normative):
- `run_id`: a stable, opaque per-run correlation id.
- `trace_context`: a small carrier for distributed tracing headers (W3C Trace Context + optional baggage).
- `tags`: bounded key/value annotations for correlation (e.g., workflow ids, repo ids).

Closed shape (v1, normative):
- `obs.schema` MUST be exactly `"agent_api.obs.v1"`.
- Unknown keys in the `obs` object MUST NOT be emitted.
- If `trace_context` is an object, unknown keys inside it MUST NOT be emitted.

### Bounds (v1, normative)

In addition to the global `data` 64 KiB bound, implementations MUST enforce all of the following when emitting an obs facet:

- `run_id`:
  - MUST NOT contain `\n` or `\r`
  - bounded length: `len(run_id) <= 128` (bytes, UTF-8)
- `trace_context` (when non-null):
  - each field (`traceparent`, `tracestate`, `baggage`) MUST be either `null` or a string that:
    - MUST NOT contain `\n` or `\r`
    - is bounded length:
      - `len(traceparent) <= 256` (bytes, UTF-8)
      - `len(tracestate) <= 1024` (bytes, UTF-8)
      - `len(baggage) <= 2048` (bytes, UTF-8)
- `tags` (when non-null):
  - MUST be a JSON object whose keys and values are strings
  - entry count bound: `count(tags) <= 32`
  - each tag key:
    - MUST match regex: `^[a-z][a-z0-9_.-]*$`
    - bounded length: `len(key) <= 64` (bytes, UTF-8)
  - each tag value:
    - MUST NOT contain `\n` or `\r`
    - bounded length: `len(value) <= 256` (bytes, UTF-8)

Obs facet bound enforcement (v1, normative):
- If an emitted `run_id` or `trace_context.*` string violates these bounds, the backend MUST set that field to `null`.
- If `trace_context` becomes an object with all-null fields after enforcement, the backend SHOULD set `trace_context` to `null`.
- If an emitted `tags` map violates these bounds:
  - entries with invalid keys/values (wrong type, regex mismatch, or length violations) MUST be dropped, and
  - if the entry count still exceeds 32, the backend MUST drop entries deterministically per the merge/precedence rules below.
  - if all entries are dropped, the backend SHOULD set `tags` to `null` (rather than emitting an empty object).

### Merge / precedence (v1, normative)

Implementations may have multiple potential sources of obs data for a run (e.g., caller-provided context via extension keys,
backend-generated ids, backend-provided trace carriers).

When computing the effective `obs` object for emission, implementations MUST apply the following merge rules:

- `run_id`: if multiple sources provide a non-null `run_id`, the caller-provided value (if any) MUST take precedence.
- `trace_context`: merge per-field; for each field, a non-null caller-provided value (if any) MUST take precedence over
  a backend-provided value.
- `tags`: merge as a map with stable precedence:
  - caller tags take precedence on key conflicts (caller value wins),
  - if a tags entry-count bound would be exceeded, the implementation MUST retain caller-provided tags first (sorted by key),
    then fill remaining capacity with backend-provided tags (sorted by key).

### Relationship to upcoming `agent_api.obs.*` surfaces (v1, normative)

This obs facet schema (`agent_api.obs.v1`) is the single canonical carrier location for the following planned universal
surfaces (capability ids and/or extension keys defined in their respective owner docs):

- `agent_api.obs.run_id.v1` MUST populate `obs.run_id`.
- `agent_api.obs.trace_context.v1` MUST populate `obs.trace_context`.
- `agent_api.obs.tags.v1` MUST populate `obs.tags`.

Any backend that supports any `agent_api.obs.*` capability that results in emitting obs metadata MUST also advertise
`agent_api.obs.v1` and MUST emit obs metadata using the `data.obs` location defined in this spec.

Safety (v1, normative):
- The obs facet is metadata-only.
- The obs facet MUST NOT include raw backend lines or raw tool inputs/outputs in v1.

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
