# Wrapper Events Ingestion Contract (v1)

Status: **Normative**  
Scope: wrapper-level ingestion of JSONL/NDJSON / line-oriented outputs

This document defines a zero-ambiguity contract for ingesting wrapper output streams in a safe,
bounded-memory way. It is intentionally **not** a Substrate envelope contract.

## RFC 2119 language

This document uses requirement keywords (`MUST`, `MUST NOT`). Any change to these requirements
requires updating this spec and reviewing ADR 0007.

## Purpose

Provide a shared ingestion boundary for wrapper streams:

- bounded memory (oversize lines discarded without buffering)
- per-line isolation (errors don’t stop the run)
- raw retention off by default (explicit capture knobs + budgets)
- adapter opt-in (feature-gated wrappers)

## Crate and features (normative)

The workspace MUST provide a crate named `wrapper_events` at:

- `crates/wrapper_events`

The crate MUST expose feature flags:

- `default = []`
- `codex`: enables Codex adapter (pulls `crates/codex`)
- `claude_code`: enables Claude Code adapter (pulls `crates/claude_code`)
- `tokio`: enables async ingestion helpers only

The base crate MUST compile without tokio and without either adapter enabled.

## Constants (normative)

Ingestion MUST use a fixed read chunk size:

- `CHUNK_SIZE_BYTES = 8192`

## Bounded line reader (normative)

Both sync and async ingestion MUST implement the same bounded algorithm:

1. Read fixed-size chunks of exactly 8192 bytes (except final short read).
2. Scan for `\\n` delimiters.
3. Track the number of bytes observed since the last `\\n`.
4. Never allocate/store a line beyond `max_line_bytes`.
5. If a line exceeds `max_line_bytes`, enter discard-mode until the next `\\n` is seen.

### Explicit prohibitions (normative)

- Sync ingestion MUST NOT use `BufRead::read_line` or `read_until`.
- Async ingestion MUST NOT use `tokio::io::AsyncBufReadExt::lines()`, `read_line`, or `read_until`.

### Oversized line record (normative)

If a physical line exceeds `max_line_bytes`, ingestion MUST:

- discard bytes until the next `\\n` (without storing the oversized content)
- emit one record for that physical line with:
  - `LineRecordError::LineTooLong { observed_bytes, max_line_bytes }`
  - `captured_raw = None` regardless of capture settings

### Line normalization (normative)

For each extracted logical line:

- `\\n` delimiter is not included
- exactly one trailing `\\r` MUST be removed if present
- full `.trim()` MUST NOT be applied
- whitespace-only lines MUST be skipped (no record emitted)

## Config vocabulary (normative)

The crate MUST define:

- `IngestLimits { max_line_bytes, max_raw_bytes_total }`
- `CaptureRaw = None | Line | Json | Both`
- `ErrorDetailCapture = RedactedSummaryOnly | FullDetails`
- `IngestConfig { limits, capture_raw, error_detail_capture, error_sink }`

Defaults MUST be:

- `capture_raw = None`
- `error_detail_capture = RedactedSummaryOnly`
- `error_sink = None`

If `error_detail_capture = FullDetails` and `error_sink = None`, ingestion MUST proceed and emit
only redacted record errors; no full details are emitted anywhere.

## Parsing interface (normative)

The crate MUST define:

```rust
pub struct LineInput<'a> {
    pub line: &'a str,
    pub json_capture: Option<&'a serde_json::Value>,
}
```

Normative rule:

- `json_capture` is an optimization-only hint. Enabling JSON capture MUST NOT change parsing
  results (success/failure classification or produced typed event values), except for the
  presence/absence of `CapturedRaw.json`.

The crate MUST define:

```rust
pub trait LineParser {
    type Event;
    type Error: ClassifiedParserError;
    fn reset(&mut self);
    fn parse_line(&mut self, input: LineInput<'_>) -> Result<Option<Self::Event>, Self::Error>;
}
```

And:

```rust
pub enum AdapterErrorCode { JsonParse, Normalize, TypedParse, Unknown }

pub trait ClassifiedParserError: std::error::Error {
    fn code(&self) -> AdapterErrorCode;
    fn redacted_summary(&self) -> String; // MUST NOT include raw line contents
    fn full_details(&self) -> String;     // MAY include raw
}
```

## Error secrecy + full details sink (normative)

Record errors MUST be safe-by-default:

- record errors MUST NOT embed raw line contents
- adapters MUST provide redacted summaries that do not include raw lines

Full details are emitted only via a sink:

```rust
pub trait ErrorDetailSink: Send + 'static {
    fn on_error(&mut self, detail: ErrorDetail);
}
```

If `error_detail_capture = FullDetails` and `error_sink.is_some()`, ingestion MUST call `on_error`
exactly once per adapter error, from the ingestion task/thread.

## Records and capture semantics (normative)

The crate MUST define:

```rust
pub struct CapturedRaw {
    pub line: Option<String>,
    pub json: Option<serde_json::Value>,
}
```

Capture semantics:

- `CapturedRaw.line`:
  - captured before parsing when `CaptureRaw` includes `Line`
  - present even if parsing fails
  - never captured for `LineTooLong`
- `CapturedRaw.json`:
  - JSON capture parses the line solely for capture
  - present only if JSON capture parse succeeds
  - may be present even if adapter parsing fails later
  - JSON capture failures MUST NOT affect parsing outcome

## Raw capture budget (normative)

Budget counting:

- `line` bytes: `line.as_bytes().len()`
- `json` bytes: `serde_json::to_vec(&value).len()`

When a capture would exceed remaining budget:

- that capture MUST be skipped (no truncation)

For `CaptureRaw::Both`, capture order MUST be:

1. line (if it fits)
2. json (if it fits in remaining budget)

No separate suppression error is emitted when capture is skipped due to budget.

## Normalized event minimum fields (normative)

The crate MUST define a minimal normalized envelope:

- `WrapperAgentKind` (at least `Codex`, `ClaudeCode`)
- `NormalizedEventKind` (at least: `TextOutput`, `ToolCall`, `ToolResult`, `Status`, `Error`, `Unknown`)
- `NormalizationContext` (consumer-supplied attribution)
- `ValidatedChannelString` with:
  - max length 64 bytes
  - ASCII only
  - allowed chars regex `^[A-Za-z0-9][A-Za-z0-9._/-]{0,63}$`
  - invalid input dropped (no truncation)

And:

```rust
pub struct NormalizedWrapperEvent {
    pub line_number: usize,
    pub agent_kind: WrapperAgentKind,
    pub kind: NormalizedEventKind,
    pub context: NormalizationContext,
    pub channel: Option<ValidatedChannelString>,
    pub captured_raw: Option<CapturedRaw>,
}
```

## Adapter deliverables (normative)

- With feature `codex`, the crate MUST provide a Codex adapter that uses Codex’s existing parsing
  semantics.
- With feature `claude_code`, the crate MUST provide a Claude Code adapter that is stateful and
  line-oriented (no batch parsing requirement for ingestion).

