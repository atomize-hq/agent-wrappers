# C0 Spec — `claude_code` streaming print stream-json API

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Scope

Implement the streaming API described in ADR-0010 in `crates/claude_code` so consumers can observe
typed stream-json events incrementally as stdout produces JSONL.

In-scope:
- Add `ClaudeClient::print_stream_json(...)` (exact signature and types pinned in `contract.md`).
- Start `claude --print --output-format stream-json` and yield items without waiting for process exit.
- Line framing rules:
  - tolerate CRLF by stripping trailing `\r` before JSON parse
  - skip blank lines
- Parse policy (DR-0002/DR-0003):
  - stream items are `Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>` in-order
  - parse errors are redacted (no raw line content embedded)
- Stderr handling (DR-0010): discard stderr by default (no buffering); optional mirror allowed but never retained.
- Backpressure (DR-0009 + protocol spec): bounded channel capacity `32`, block on send (no drops).
- Timeout/cancellation: align with `tokio::time::timeout` + `kill_on_drop(true)` semantics pinned in `stream-json-print-protocol-spec.md`.
- Add the feature-local CI workflow + smoke scripts used by the checkpoint:
  - `.github/workflows/claude-code-live-stream-json-smoke.yml`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/*`

## Acceptance Criteria

- `crates/claude_code` exports a streaming print API per ADR-0010 and compiles on Linux/macOS/Windows.
- Streaming yields at least one item before process exit when provided a synthetic stdout source (tests prove incrementality; no real `claude` binary required).
- CRLF + blank-line handling matches the framing rules above.
- Parse errors are redacted and do not include raw backend lines.
- Feature-local smoke workflow exists and runs the smoke scripts on ubuntu/macos/windows; also runs `make preflight` on ubuntu.

## Out of Scope

- Any `crates/agent_api` wiring (that is C1).
- Any requirement to run a real `claude` binary in CI.
- Interactive/TUI mode or non-`--print` flows.
