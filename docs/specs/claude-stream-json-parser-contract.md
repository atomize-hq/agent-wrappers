# Claude stream-json Parser Contract (v1)

Status: **Normative** (paired with ADR 0008)  
Scope: offline parsing of Claude Code `--output-format=stream-json` logs

## Normative language

This document uses RFC 2119 requirement keywords (`MUST`, `MUST NOT`). Any change to these
requirements requires updating this spec and reviewing ADR 0008.

## Purpose

Define a zero-ambiguity public API for parsing Claude Code stream-json output (JSONL/NDJSON) into a
typed, lossless raw event model, with per-line isolation and forward compatibility.

This contract is wrapper-level and Claude-specific. It is not a Substrate envelope contract.

## Public API (v1, normative)

The `claude_code` crate MUST expose the following items at the crate root:

```rust
use claude_code::{
    ClaudeStreamEvent, ClaudeStreamJsonErrorCode, ClaudeStreamJsonEvent, ClaudeStreamJsonParseError,
    ClaudeStreamJsonParser,
};
```

### Parser

`ClaudeStreamJsonParser` MUST be stateful and MUST provide:

```rust
impl ClaudeStreamJsonParser {
    pub fn new() -> Self;
    pub fn reset(&mut self);
    pub fn parse_line(&mut self, line: &str)
        -> Result<Option<ClaudeStreamJsonEvent>, ClaudeStreamJsonParseError>;
    pub fn parse_json(&mut self, value: &serde_json::Value)
        -> Result<Option<ClaudeStreamJsonEvent>, ClaudeStreamJsonParseError>;
}
```

### Typed raw event model

The typed model MUST be lossless: all variants retain `raw: Value`.

```rust
pub struct ClaudeStreamEvent {
    pub event_type: String,
    pub raw: serde_json::Value,
}

pub enum ClaudeStreamJsonEvent {
    SystemInit { session_id: String, raw: serde_json::Value },
    SystemOther { session_id: String, subtype: String, raw: serde_json::Value },

    UserMessage { session_id: String, raw: serde_json::Value },
    AssistantMessage { session_id: String, raw: serde_json::Value },

    ResultSuccess { session_id: String, raw: serde_json::Value },
    ResultError { session_id: String, raw: serde_json::Value },

    StreamEvent { session_id: String, stream: ClaudeStreamEvent, raw: serde_json::Value },

    Unknown { session_id: Option<String>, raw: serde_json::Value },
}
```

## Line handling (normative)

Before JSON parsing, `parse_line` MUST:

- Strip exactly one trailing `\r` if present.
- Ignore whitespace-only lines (return `Ok(None)`).
- MUST NOT apply full `.trim()` before JSON parsing.

## Outer JSON paths (normative)

All paths below are over the parsed object `raw`.

### Outer type

`raw["type"]` MUST be a string for all parsed objects. If missing/not string → `TypedParse`.

Known outer type strings (v1):

- `"system"`
- `"user"`
- `"assistant"`
- `"result"`
- `"stream_event"`

If `raw["type"]` is a string but not one of these → return `Ok(Some(Unknown{..}))` (not an error).

### session_id

For known outer types, session id MUST be extracted from the first present valid string:

1. `raw["session_id"]`
2. `raw["sessionId"]` (alias)

If missing/not string → `TypedParse`.

For unknown outer types, session id extraction is best-effort and may be `None`.

### system subtype

For `type == "system"`:

- `raw["subtype"]` MUST be a string; otherwise `TypedParse`.
- If subtype is `"init"` → `SystemInit`.
- Otherwise → `SystemOther{subtype}` (not an error).

### stream_event wrapper

For `type == "stream_event"`:

- `raw["event"]` MUST be an object; otherwise `TypedParse`.
- `raw["event"]["type"]` MUST be a string; otherwise `TypedParse`.

Unknown inner `raw["event"]["type"]` strings are allowed and MUST still yield `StreamEvent`.

### result discriminator

For `type == "result"`:

- `raw["subtype"]` MUST be a string; otherwise `TypedParse`.
- subtype MUST be `"success"` or `"error"`; otherwise `TypedParse`.

Optional cross-check:

- If `raw["is_error"]` is present, it MUST be boolean; otherwise `TypedParse`.
- If present and inconsistent with subtype, parser MUST return `Normalize`:
  - subtype `"success"` with `is_error==true` → `Normalize`
  - subtype `"error"` with `is_error==false` → `Normalize`

`Normalize` MUST NOT be emitted for any other condition in v1.

## Error taxonomy (normative)

`ClaudeStreamJsonParseError.code` MUST be one of:

- `JsonParse` (invalid JSON in `parse_line`)
- `TypedParse` (required path missing/invalid for a known variant)
- `Normalize` (result subtype cross-check inconsistency only)
- `Unknown` (reserved; not required in v1)

### Redaction (normative)

`ClaudeStreamJsonParseError.message` MUST be redacted (it MUST NOT embed the full raw line).

## Optimization neutrality (normative)

Let a logical line `L` parse as JSON value `V`. Then:

- If `parse_line(L)` returns `Ok(Some(E))`, `parse_json(&V)` MUST return `Ok(Some(E))` with the same
  variant and extracted fields.
- If `parse_line(L)` returns `Err` with `code` in `{TypedParse, Normalize}`, then `parse_json(&V)`
  MUST return `Err` with the same `code` and the same invariant decision.
- `parse_json` MUST NOT emit `JsonParse` (JSON parsing already occurred).

