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

## Framing (JSONL)

Normative rules:
- Each JSON value is delimited by newline boundaries (JSONL).
- Blank lines MUST be ignored.
- CRLF tolerance: a trailing `\r` MUST be stripped before JSON parsing.
- Ordering MUST be preserved: events and redacted parse errors are yielded in the order observed on stdout.

## Parse error behavior

Normative rules:
- Parse errors MUST be represented as redacted outcomes on the same stream (in-order).
- The stream SHOULD continue after a parse error where feasible.
- Raw line content MUST NOT be embedded in the error message by default.

## Backpressure

Normative rule:
- Backpressure is applied (no silent drops). If the consumer is slow, the reader may block.

## Cancellation / timeout (high level)

Normative rules:
- Timeout and cancellation behavior must be deterministic and consistent across platforms within the allowed envelope.
- Completion signaling MUST respect DR-0012 when used via `agent_api`.

