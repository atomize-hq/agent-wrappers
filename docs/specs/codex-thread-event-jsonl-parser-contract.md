# Codex ThreadEvent JSONL Parser Contract (v1)

Status: **Normative** (paired with ADR 0005)  
Scope: offline parsing of Codex `--json` JSONL logs into `ThreadEvent`

## Normative language

This document uses RFC 2119-style requirement keywords (`MUST`, `MUST NOT`).
Any change to these requirements requires updating this spec and reviewing ADR 0005.

## Purpose

Define a **zero-ambiguity** public API for parsing Codex CLI JSONL logs into the existing typed
event model (`ThreadEvent`) while **reusing the same normalization semantics** as the streaming
path.

This contract is intentionally scoped to Codex-specific JSONL parsing. Higher-level cross-agent
normalization, analytics, and reporting are out of scope.

## Normative references

- Normalization semantics: `crates/codex/JSONL_COMPAT.md` (authoritative)
- Motivation and scope: `docs/adr/0005-codex-jsonl-log-parser-api.md`

If there is any conflict between ADR 0005 and this contract, this contract takes precedence.

## Public API (v1, normative)

The `codex` crate MUST expose the following items at the crate root (i.e., these paths MUST
resolve for downstream consumers):

```rust
use codex::{
    JsonlThreadEventParser, ThreadEventJsonlFileReader, ThreadEventJsonlReader,
    ThreadEventJsonlRecord, thread_event_jsonl_file, thread_event_jsonl_reader,
};
```

Concretely, the crate MUST make these symbols available as:

- `codex::JsonlThreadEventParser`
- `codex::ThreadEventJsonlRecord`
- `codex::ThreadEventJsonlReader`
- `codex::ThreadEventJsonlFileReader`
- `codex::thread_event_jsonl_reader`
- `codex::thread_event_jsonl_file`

The `codex` crate MUST provide a `codex::jsonl` module containing the following API surface:

```rust
pub mod jsonl {
    use std::io::BufRead;
    use std::path::Path;

    use crate::{CodexError, ExecStreamError, ThreadEvent};

    #[derive(Clone, Debug, Default)]
    pub struct JsonlThreadEventParser { /* private */ }

    impl JsonlThreadEventParser {
        /// Constructs a new parser with no established context.
        pub fn new() -> Self;

        /// Clears any thread/turn context and resets synthetic turn counters.
        pub fn reset(&mut self);

        /// Parses a single logical JSONL line.
        ///
        /// - Returns `Ok(None)` for empty / whitespace-only lines.
        /// - Otherwise returns `Ok(Some(ThreadEvent))` on success.
        /// - Returns `Err(ExecStreamError)` on JSON parse / normalization / typed parse failures.
        pub fn parse_line(&mut self, line: &str) -> Result<Option<ThreadEvent>, ExecStreamError>;
    }

    #[derive(Clone, Debug)]
    pub struct ThreadEventJsonlRecord {
        /// 1-based line number in the underlying source (file/reader).
        pub line_number: usize,
        /// The parse outcome for this line (success or failure).
        pub outcome: Result<ThreadEvent, ExecStreamError>,
    }

    pub struct ThreadEventJsonlReader<R: BufRead> { /* private */ }

    impl<R: BufRead> ThreadEventJsonlReader<R> {
        /// Creates a reader-backed iterator with a fresh parser.
        pub fn new(reader: R) -> Self;

        /// Consumes the iterator and returns the wrapped reader.
        pub fn into_inner(self) -> R;
    }

    impl<R: BufRead> Iterator for ThreadEventJsonlReader<R> {
        type Item = ThreadEventJsonlRecord;
    }

    pub type ThreadEventJsonlFileReader =
        ThreadEventJsonlReader<std::io::BufReader<std::fs::File>>;

    /// Convenience constructor for reader-backed parsing.
    pub fn thread_event_jsonl_reader<R: BufRead>(reader: R) -> ThreadEventJsonlReader<R>;

    /// Convenience constructor for file-backed parsing.
    ///
    /// This function MUST:
    /// - open the file at `path`,
    /// - wrap it in `std::io::BufReader`,
    /// - return a `ThreadEventJsonlFileReader`.
    ///
    /// I/O failures (open errors) MUST be surfaced as
    /// `Err(ExecStreamError::Codex(CodexError::CaptureIo(io_err)))`.
    pub fn thread_event_jsonl_file(
        path: impl AsRef<Path>,
    ) -> Result<ThreadEventJsonlFileReader, ExecStreamError>;
}
```

### Async I/O (v1 decision)

v1 MUST be synchronous (`std::io::BufRead`) to keep the API small and deterministic.
Async reader support (e.g., `tokio::io::AsyncBufRead`) is explicitly deferred to a future v2/vNext
contract if needed.

## Parsing semantics (normative)

### Line normalization

Before attempting JSON parsing, the parser MUST apply the same line-handling rules as
`crates/codex/JSONL_COMPAT.md`:

- Empty / whitespace-only lines MUST be ignored.
- A single trailing `\r` MUST be trimmed (CRLF tolerance).

The `JsonlThreadEventParser::parse_line` method MUST implement this behavior and return `Ok(None)`
for ignored lines.

The parser MUST NOT apply a full `.trim()` before JSON parsing.

### Event normalization and context inference

For non-empty lines, the parser MUST apply the event normalization rules defined in
`crates/codex/JSONL_COMPAT.md` (including event type aliases, item envelope normalization, field
aliases, and context inference).

The offline parsing API MUST match the streaming normalization behavior:

- Context is maintained across lines (thread/turn inference).
- Observing `thread.started` or `thread.resumed` MUST reset current turn context (per
  `crates/codex/JSONL_COMPAT.md`).
- Errors MUST be per-line and MUST NOT prevent parsing of later lines.

### Iterator behavior

`ThreadEventJsonlReader<R>` MUST:

- Read lines from the underlying `BufRead` source in order.
- Maintain a 1-based physical line counter.
- For each physical line:
  - If the line is empty/whitespace-only (after trimming `\r`), it MUST advance the line counter and
    emit no record for that line.
  - Otherwise, it MUST emit exactly one `ThreadEventJsonlRecord` with:
    - `line_number` set to the physical line number, and
    - `outcome` set to the parse result for that line.

Note: This makes `line_number` values potentially non-contiguous in the emitted record stream when
the input contains blank lines. This is intentional and is part of the contract.

### Diagnostics linkage (normative)

`ThreadEventJsonlRecord.line_number` is the authoritative physical line reference for downstream
diagnostics and reporting. Acceptance scenarios MUST assert the field’s value (including gaps when
blank lines are skipped) so tooling can surface precise locations for parse failures and recovery.

## Data minimization (v1, normative)

To keep the parsing surface safe-by-default and lightweight:

- `ThreadEventJsonlRecord` MUST NOT include the raw JSONL line on success (no `raw_line` field).
- On failure, the error value SHOULD retain the original line content when possible (existing
  `ExecStreamError` variants already carry `line` for parse/normalize failures).

If a downstream tool needs a “raw log viewer”, it should read the file directly or wrap the reader
to retain raw lines locally, rather than expanding the wrapper’s core parsing record.

## Stability and compatibility guarantees (normative)

- The offline parser MUST reuse the same normalization behavior as streaming; drift between the two
  is a bug.
- Accepting new upstream JSON shapes SHOULD be handled by extending normalization and/or adding
  aliases rather than breaking signatures.
- Breaking changes to this public API require a semver-major bump of the `codex` crate.

## Strict mode (v1 decision)

v1 intentionally provides a tolerant, per-line outcome stream only.

- The wrapper MUST NOT provide separate “strict mode” helpers in v1 (e.g., `*_strict` constructors
  that stop on the first error or return only `Result<ThreadEvent, _>` without line numbers).
- If strict helpers are added in the future, they require a contract update and MUST remain
  expressible as a deterministic wrapper around the tolerant iterator.
