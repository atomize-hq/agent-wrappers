# Protocol Spec — `claude --print --output-format stream-json` (live)

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Purpose

This spec pins the observable semantics of reading Claude’s stream-json output live and converting it
into typed events.

Source ADR:
- `docs/adr/0010-claude-code-live-stream-json.md`

## Spawn contract

The implementation MUST spawn Claude with:
- `--print`
- `--output-format stream-json`

The streaming implementation MUST NOT require a PTY. Stdout must be read via a pipe.

Implementation structure (normative; align with existing repo patterns):
- Use `tokio::process::Command`.
- Set `stdout` to `Stdio::piped()` and read via `tokio::io::BufReader` + `AsyncBufReadExt::lines()`.
- Apply `Command::kill_on_drop(true)` so cancellation/timeout tears down the child deterministically (Codex backend pattern).
- Emit items through a bounded `tokio::sync::mpsc` channel and expose a `futures_core::Stream` wrapper over the receiver.

## Framing (JSONL)

Normative rules:
- Each JSON value is delimited by newline boundaries (JSONL).
- Blank lines MUST be ignored.
- CRLF tolerance: a trailing `\r` MUST be stripped before JSON parsing.
- Ordering MUST be preserved: events and redacted parse errors are yielded in the order observed on stdout.

## Parse error behavior

Normative rules:
- Parse errors MUST be represented as redacted outcomes on the same stream (in-order).
- The stream MUST continue after a parse error (do not fail-fast on a single bad line).
- Raw line content MUST NOT be embedded in the error message by default.

## Backpressure

Normative rule:
- Backpressure is applied (no silent drops).

Pinned v1 policy:
- Stream channel capacity MUST be `32` items (aligns with `crates/agent_api` backend channels).
- When the channel is full, the reader task MUST await send (i.e., block) rather than drop.
- If the receiver is dropped, sends fail and the reader task MUST terminate the child process by dropping the
  `tokio::process::Child` handle that was spawned with `kill_on_drop(true)`, then exit.

## Cancellation / timeout (high level)

Normative rules:
- Timeout behavior MUST be deterministic:
  - Timeout starts after successful process spawn.
  - If the timeout elapses, the process MUST be terminated by dropping the `Child` handle spawned with
    `kill_on_drop(true)`, and the completion future resolves to `Err(ClaudeCodeError::Timeout { timeout })`.
- Cancellation behavior MUST be deterministic:
  - Dropping the `events` stream is treated as cancellation for the streaming handle and MUST terminate the child process
    by dropping the `Child` handle spawned with `kill_on_drop(true)`.
- Completion signaling MUST respect Universal Agent API DR-0012 when used via `agent_api`:
  - `agent_api` completion must not resolve until the universal event stream is final (or dropped).

## Stderr handling (deadlock avoidance)

Normative rules:
- stderr MUST NOT be buffered into memory by the streaming API.
- Default behavior MUST discard stderr (`Stdio::null()`).
- Optional behavior: if `ClaudeClientBuilder::mirror_stderr(true)` is set, stderr MUST be piped and copied to the parent
  process stderr, but MUST NOT be retained.
