# JSONL Compatibility (Codex CLI parity)

This crate consumes Codex CLI `--json` output as a JSONL stream (one JSON object per stdout line).
The streaming APIs `CodexClient::stream_exec` and `CodexClient::stream_resume` yield a stream of
`Result<ThreadEvent, ExecStreamError>`.

## Goals
- Prefer typed `ThreadEvent` parsing for ergonomics.
- Tolerate upstream drift (field renames, nesting changes, missing context) via normalization and
  unknown-field capture.
- Do not terminate the entire stream on the first malformed/unrecognized line when it is possible
  to continue.

## Normalization (what + when)

Normalization is applied to every non-empty JSONL line before attempting to deserialize it into
`ThreadEvent`. It is heuristic-driven (based on which fields are present/missing), not gated on an
explicit Codex CLI version.

### Event type aliases
The parser accepts multiple upstream names for the same typed variants:
- `thread.started` and `thread.resumed` → `ThreadEvent::ThreadStarted`
- `item.started` and `item.created` → `ThreadEvent::ItemStarted`
- `item.delta` and `item.updated` → `ThreadEvent::ItemDelta`

### Context inference (thread/turn ids)
The stream maintains the most recently observed `thread_id` and `turn_id`:
- If a `turn.*` or `item.*` event is missing `thread_id` and a prior `thread.started`/`thread.resumed`
  established context, the missing `thread_id` is filled from context.
- If an `item.*` event is missing `turn_id` and a prior `turn.started` established context, the
  missing `turn_id` is filled from context.
- If `turn.started` is missing `turn_id`, a synthetic id `synthetic-turn-N` is generated.

### Item envelope normalization
For `item.*` events, older Codex CLI versions may nest item fields under `{"item": {...}}`. The
parser normalizes by:
- Moving fields from `item` to the top level.
- Renaming `item.type` → `item_type`.
- If `item_type` is command-shaped and `content` is missing, synthesizing `content` from legacy
  fields like `text`, `command`, `aggregated_output`, `exit_code`, and `stderr`.

For delta-shaped item events (`item.delta` / `item.updated`), if `delta` is absent but `content` is
present, `content` is treated as the delta payload (`content` → `delta`).

### Field aliases
Common legacy field names are accepted during deserialization:
- `item_id`: `item_id` or `id`
- text deltas: `text` or `text_delta`
- command/file output: `aggregated_output` / `output` → `stdout`; `error_output` / `err` → `stderr`
- file change: `file_path` → `path`; `patch` → `diff`
- MCP tool call: `server` → `server_name`; `tool` → `tool_name`

## Unknown field capture
Many event/payload structs include an `extra: BTreeMap<String, serde_json::Value>` field annotated
with `#[serde(flatten)]`. Any fields not understood by the typed schema are preserved there (rather
than dropped silently) so callers can inspect or forward them.

## Error surfacing and stream behavior
- Each JSONL line produces exactly one stream item:
  - `Ok(ThreadEvent)` when the line normalizes and deserializes successfully.
  - `Err(ExecStreamError)` when JSON parsing, normalization, or typed deserialization fails.
- Malformed/unrecognized lines do not stop the stream; subsequent lines are still read and emitted
  when possible.
- The stream can still terminate early for transport-level issues (e.g., stdout read failures or the
  Codex process exiting).
