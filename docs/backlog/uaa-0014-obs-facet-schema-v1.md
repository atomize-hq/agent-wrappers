# uaa-0014 — Obs facet schema (events + completion)

Status: Draft (backlog-only; not part of canonical Universal Agent API specs)

This document captures draft spec text for an `agent_api.obs.v1` facet that was intentionally
removed from the canonical specs because it may never ship. If/when we decide to implement obs,
this text can be ported into the canonical contract docs.

Why backlog-only:
- The Substrate integration pattern treats Substrate as the owner of correlation and routing
  context, and does not require wrappers to invent envelope correlation fields.

## Proposed canonical edits (if revived)

Target docs:
- `docs/specs/universal-agent-api/event-envelope-schema-spec.md`
- `docs/specs/universal-agent-api/capabilities-schema-spec.md`

### Capabilities — add `agent_api.obs.v1`

Add to `docs/specs/universal-agent-api/capabilities-schema-spec.md` under "Standard capability ids":

- `agent_api.obs.v1`:
  - The backend may emit the bounded observability facet (`data.obs`) on `AgentWrapperEvent.data`
    and/or `AgentWrapperCompletion.data` per `event-envelope-schema-spec.md` ("Obs facet (v1)").
  - A backend that emits an obs facet MUST advertise this capability.
  - If the backend emits an obs facet, it MUST be nested under `data.obs` (never as `data.schema="agent_api.obs.v1"`)
    so that it can coexist with other facets such as `agent_api.tools.structured.v1`.

### Event envelope — tools facet coexistence tweak

In `docs/specs/universal-agent-api/event-envelope-schema-spec.md` under "Tools facet (structured.v1)":

- Tighten wording to require that `ToolCall` / `ToolResult` `data` is a JSON object containing at minimum:
  `{ "schema": "agent_api.tools.structured.v1", "tool": { ... } }`.
- Update the schema example to show an optional sibling `obs` field so tools facet stays the owner
  of `data.schema` while obs metadata coexists safely:

```json
{
  "schema": "agent_api.tools.structured.v1",
  "tool": { "...": "..." },
  "obs": { "schema": "agent_api.obs.v1", "...": "..." }
}
```

## Draft spec text — Obs facet (v1)

The following is the draft normative text for insertion into
`docs/specs/universal-agent-api/event-envelope-schema-spec.md`.

---

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

