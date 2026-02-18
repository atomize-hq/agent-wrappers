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

Normative requirements:
- Events MUST be yielded incrementally as stdout produces JSONL (no “buffer-until-exit”).
- Stream item type MUST preserve in-order parse errors as redacted outcomes (see DR-0002/DR-0003).
- CRLF tolerance: trailing `\r` is stripped before JSON parsing.
- Safety posture: MUST NOT embed raw backend lines in errors or in `agent_api` event data by default.

## `agent_api` observable behavior

Normative requirements:
- Claude backend advertises capability id: `agent_api.events.live`.
- Universal event stream is live: at least one `AgentWrapperEvent` may be observed before the Claude process exits.
- Completion MUST obey DR-0012 semantics (completion waits for stream finality or stream drop).

## Error taxonomy (high level)

Streaming errors are categorized as:
- spawn failures (process cannot be started)
- I/O read failures
- per-line parse failures (redacted; stream continues where feasible)
- timeout / cancellation

## Non-goals

- Upstream `claude` CLI flags or behavior changes.
- Any interactive/TUI mode support.
- Any requirement to buffer or return raw stdout/stderr.

