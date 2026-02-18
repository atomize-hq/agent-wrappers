# Platform Parity Spec — Live stream-json stdout framing

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Purpose

This spec pins the allowed behavior envelope across Linux/macOS/Windows for:
- process spawning
- stdout newline framing (LF vs CRLF)
- cancellation behavior

Source ADR:
- `docs/adr/0010-claude-code-live-stream-json.md`

## Requirements

- All supported OSes MUST pass the feature-local smoke workflow at the CI checkpoint:
  - `.github/workflows/claude-code-live-stream-json-smoke.yml`
- Newline framing MUST be correct on all OSes:
  - CRLF is tolerated by stripping a trailing `\r` before parse.
- The implementation MUST avoid deadlocks:
  - stderr is discarded by default (`Stdio::null()`), and MUST NOT be buffered into memory by the streaming API.

## Cancellation parity (normative)

- Cancellation mechanism MUST be the same across platforms:
  - the child process is spawned with `kill_on_drop(true)`
  - dropping the `events` receiver triggers termination by dropping the `Child` handle
- Allowed completion outcomes on cancellation:
  - completion is allowed to resolve to `Ok(ExitStatus)` (process observed to exit after termination), or
  - completion is allowed to resolve to `Err(ClaudeCodeError::Wait(_))` in rare OS-specific cases.
  - timeout MUST resolve to `Err(ClaudeCodeError::Timeout { timeout })`.

## Backpressure parity (normative)

- Channel capacity is pinned at `32`.
- When the consumer is slow, stdout draining can pause due to backpressure; this is expected and allowed on all OSes.

## Evidence

Required evidence (checkpoint):
- workflow run id/link and tested SHA recorded in `session_log.md`
- pass/fail for ubuntu/macos/windows smoke jobs
