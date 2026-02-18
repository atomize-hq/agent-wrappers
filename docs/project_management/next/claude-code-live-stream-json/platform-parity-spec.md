# Platform Parity Spec — Live stream-json stdout framing

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Purpose

This spec pins the allowed behavior envelope across Linux/macOS/Windows for:
- process spawning
- stdout newline framing (LF vs CRLF)
- best-effort cancellation behavior

Source ADR:
- `docs/adr/0010-claude-code-live-stream-json.md`

## Requirements

- All supported OSes MUST pass the feature-local smoke workflow at the CI checkpoint:
  - `.github/workflows/claude-code-live-stream-json-smoke.yml`
- Newline framing MUST be correct on all OSes:
  - CRLF is tolerated by stripping a trailing `\r` before parse.
- The implementation MUST avoid deadlocks:
  - stderr is drained (discarded or mirrored), but not retained by default.

## Evidence

Required evidence (checkpoint):
- workflow run id/link and tested SHA recorded in `session_log.md`
- pass/fail for ubuntu/macos/windows smoke jobs

