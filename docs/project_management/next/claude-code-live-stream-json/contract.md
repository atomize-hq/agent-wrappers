# Contract — Claude Code live stream-json

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Purpose

This document is the authoritative, user-facing contract for ADR-0010:
- the public Rust API surface added to `crates/claude_code` for live stream-json printing
- the `crates/agent_api` observable behavior change (Claude backend emits events live and advertises `agent_api.events.live`)
- error taxonomy + redaction posture for streaming

Source ADR:
- `docs/adr/0010-claude-code-live-stream-json.md`

## Public API (Rust)

The `crates/claude_code` crate exposes a streaming API for:
- `claude --print --output-format stream-json`

Pinned v1 API surface (normative; names/signatures stable for this feature):
```rust
use std::{future::Future, pin::Pin};

use futures_core::Stream;

use claude_code::{
    ClaudeCodeError, ClaudePrintRequest, ClaudeStreamJsonEvent, ClaudeStreamJsonParseError,
};

pub type DynClaudeStreamJsonEventStream =
    Pin<Box<dyn Stream<Item = Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>> + Send>>;

pub type DynClaudeStreamJsonCompletion =
    Pin<Box<dyn Future<Output = Result<std::process::ExitStatus, ClaudeCodeError>> + Send>>;

pub struct ClaudePrintStreamJsonHandle {
    pub events: DynClaudeStreamJsonEventStream,
    pub completion: DynClaudeStreamJsonCompletion,
}

impl claude_code::ClaudeClient {
    /// Starts `claude --print --output-format stream-json` and yields events as stdout produces JSONL.
    ///
    /// This MUST NOT require the process to exit before yielding events.
    pub fn print_stream_json(
        &self,
        request: ClaudePrintRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ClaudePrintStreamJsonHandle, ClaudeCodeError>> + Send + '_>>;
}
```

Normative requirements:
- Events MUST be yielded incrementally as stdout produces JSONL (no “buffer-until-exit”).
- Stream item type MUST preserve in-order parse errors as redacted outcomes (see DR-0002/DR-0003).
- CRLF tolerance: trailing `\r` is stripped before JSON parsing.
- Safety posture: MUST NOT embed raw backend lines in errors or in `agent_api` event data by default.
- Dep/pattern alignment:
  - Use `futures_core::Stream` + `Pin<Box<dyn Stream<...>>>` (matches `crates/codex` + `crates/agent_api` patterns).
  - Use a bounded `tokio::sync::mpsc` channel to connect the reader task to the public stream (no unbounded buffering).

## `agent_api` observable behavior

Normative requirements:
- Claude backend advertises capability id: `agent_api.events.live`.
- This feature does not introduce any additional live-specific backend capability ids; existing non-live backend capability ids may remain (see DR-0006).
- Universal event stream is live:
  - For any successful run that reaches the point where Claude produces at least one stdout stream-json line,
    the `AgentWrapperRunHandle.events` stream MUST yield at least one `AgentWrapperEvent` before the child
    process exits.
- Completion MUST obey Universal Agent API DR-0012 semantics (completion waits for stream finality or stream drop).

## Error taxonomy (high level)

Streaming errors are categorized as:
- spawn failures (process cannot be started)
- I/O read failures
- per-line parse failures (redacted; stream continues)
- timeout / cancellation

Pinned semantics:
- Non-zero exit status is **not** an error for the streaming handle; completion yields `Ok(ExitStatus)` (DR-0007).
- Timeout errors surface as `Err(ClaudeCodeError::Timeout { timeout })` and MUST terminate the child process (via `kill_on_drop(true)`).
- Cancellation:
  - Dropping the `events` stream (receiver) is defined as cancellation for the streaming handle.
  - Cancellation MUST terminate the child process by dropping the `tokio::process::Child` handle created with
    `kill_on_drop(true)`.
  - On cancellation, the completion future is allowed to resolve to either:
    - `Ok(ExitStatus)` (process observed to exit after termination), or
    - `Err(ClaudeCodeError::Wait(_))` in rare OS-specific cases.
    Both outcomes are considered a successful cancellation teardown.

## Non-goals

- Upstream `claude` CLI flags or behavior changes.
- Any interactive/TUI mode support.
- Any requirement to buffer or return raw stdout/stderr.
