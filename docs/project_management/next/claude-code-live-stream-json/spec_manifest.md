# Spec Manifest — Claude Code live stream-json

Status: Draft  
Date (UTC): 2026-02-17  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## ADR Inputs

- ADR: `docs/adr/0010-claude-code-live-stream-json.md`
- Baseline (referenced; not duplicated):
  - `docs/adr/0009-universal-agent-api.md`
  - `docs/project_management/next/universal-agent-api/run-protocol-spec.md`
  - `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`
  - `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`

## Required planning artifacts (always)

- `docs/project_management/next/claude-code-live-stream-json/plan.md`
- `docs/project_management/next/claude-code-live-stream-json/tasks.json`
- `docs/project_management/next/claude-code-live-stream-json/session_log.md`

## Required cross-platform / decision-heavy artifacts

- `docs/project_management/next/claude-code-live-stream-json/decision_register.md`
- `docs/project_management/next/claude-code-live-stream-json/impact_map.md`
- `docs/project_management/next/claude-code-live-stream-json/manual_testing_playbook.md`
- Smoke scripts:
  - `docs/project_management/next/claude-code-live-stream-json/smoke/linux-smoke.sh`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/macos-smoke.sh`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/windows-smoke.ps1`

## Required spec documents (deterministic set)

### Slice specs (triads)

- `docs/project_management/next/claude-code-live-stream-json/C0-spec.md` — add a streaming `--print --output-format stream-json` API to `crates/claude_code` (no `agent_api` wiring).
- `docs/project_management/next/claude-code-live-stream-json/C1-spec.md` — wire the streaming API into `crates/agent_api` Claude backend + advertise `agent_api.events.live` + fixture/synthetic validation.

### Contract spec (user-facing)

Trigger: ADR introduces/changes a user-facing **library** contract (public Rust API + capability advertisement).

- `docs/project_management/next/claude-code-live-stream-json/contract.md` — authoritative contract for:
  - new `crates/claude_code` public streaming API surface (types, method name, returned handle shape)
  - `agent_api` behavior change: Claude backend advertises `agent_api.events.live` and emits events before process exit
  - error taxonomy and redaction rules for the streaming API
  - absence semantics (unset timeout, unset working_dir, dropped consumer stream)

### Protocol spec (streaming semantics)

Trigger: ADR introduces a stable streaming contract between a spawned CLI process and a consumer-visible event stream.

- `docs/project_management/next/claude-code-live-stream-json/stream-json-print-protocol-spec.md` — authoritative semantics for:
  - spawning `claude --print --output-format stream-json` and streaming stdout
  - framing rules (JSONL boundaries, CRLF handling, blank line handling)
  - ordering guarantees and backpressure expectations
  - per-line parse error handling (emit redacted error and continue vs fail-fast)
  - cancellation and timeout behavior

### Platform parity spec

Trigger: ADR targets multi-platform process I/O behavior (stdout streaming) and must remain correct on GitHub-hosted runners.

- `docs/project_management/next/claude-code-live-stream-json/platform-parity-spec.md` — authoritative guarantees/divergences for Linux/macOS/Windows for:
  - process spawning + stdout line framing expectations
  - newline handling (LF vs CRLF) and UTF-8 assumptions
  - cancellation behavior expectations (best-effort) by platform
  - required validation evidence

## CI checkpoint plan

Trigger: cross-platform validation is required and this feature changes process streaming behavior.

- `docs/project_management/next/claude-code-live-stream-json/ci_checkpoint_plan.md` — authoritative checkpoint grouping + required CI gates + `tasks.json` wiring.

## Coverage matrix (surface → authoritative doc)

| Surface | Owner doc |
|---|---|
| `crates/claude_code` public streaming API surface | `contract.md` |
| `crates/claude_code` streaming error model + redaction rules | `contract.md` |
| `crates/claude_code` streaming framing/ordering/backpressure/timeout/cancel semantics | `stream-json-print-protocol-spec.md` |
| `agent_api` Claude backend capability advertisement (`agent_api.events.live`) | `contract.md` |
| `agent_api` completion vs stream finality (DR-0012) | `docs/project_management/next/universal-agent-api/run-protocol-spec.md` |
| `agent_api` event envelope bounds + raw-line prohibition | `docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md` |
| Capability id naming/stability rules (`agent_api.events.live`, `backend.claude_code.*`) | `docs/project_management/next/universal-agent-api/capabilities-schema-spec.md` |
| Platform guarantees/divergences for streaming stdout | `platform-parity-spec.md` |
| Slice-specific deliverables/acceptance/out-of-scope | `C0-spec.md` / `C1-spec.md` |
| CI checkpoint grouping + CI gate commands | `ci_checkpoint_plan.md` |
| Architectural decisions (A/B pinned) | `decision_register.md` |
| Manual/non-gating validation steps | `manual_testing_playbook.md` |

## Determinism checklist (per spec)

### `contract.md`

- Names and signatures for all new public items in `crates/claude_code` (and any `agent_api` constructor/config changes, if any).
- Exactly what is considered “live streaming” and the observable guarantees.
- Absence semantics for:
  - unset timeout
  - unset working_dir
  - dropped events stream (consumer opt-out)
- Error taxonomy:
  - spawn failures
  - I/O read failures
  - per-line parse failures (redacted)
  - timeout and cancellation behavior

### `stream-json-print-protocol-spec.md`

- Exact spawn argv requirements (minimum required flags and required invariants).
- Line framing rules (newline delimiters, CRLF stripping, blank lines).
- Ordering rules (what “in order” means) and what can interleave (if anything).
- Per-line parse error policy (emit error + continue vs fail-fast) and the selected policy’s observable outcomes.
- Backpressure behavior:
  - what happens when the consumer is slow
  - buffer bounds and what “drop” means (if ever permitted)
- Timeout/cancellation semantics:
  - when timeout starts
  - what completion/error the caller observes on timeout

### `platform-parity-spec.md`

- Per-OS expectations for:
  - stdout pipe behavior and newline boundaries
  - best-effort cancellation semantics
  - known divergences (if any) and the allowed behavior envelope
- Required evidence: exactly what must pass in CI and what can be manual-only.

### `ci_checkpoint_plan.md`

- Exact workflow path(s), trigger mode(s), and required gate commands.
- Which tasks in `tasks.json` are bound to which checkpoint(s).
- Evidence requirements (e.g., linkable CI run id and tested SHA).
