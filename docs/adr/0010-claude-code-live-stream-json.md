# ADR-0010 — Claude Code live stream-json (streaming API + `agent_api` live events)
#
# Note: Run `make adr-fix ADR=docs/adr/0010-claude-code-live-stream-json.md` after editing to update
# the ADR_BODY_SHA256 drift guard.

## Status
- Status: Draft
- Date (UTC): 2026-02-17
- Owner(s): spensermcconnell

## Scope
- Feature directory: `docs/project_management/next/claude-code-live-stream-json/`
- Intended branch: `feat/claude-code-live-stream-json`
- Sequencing spine: `docs/project_management/next/sequencing.json`
- Standards:
  - `docs/project_management/task-triads-feature-setup-standard.md`
  - `docs/project_management/prompt_templates/adr-to-triad-feature-scaffold.md`

## Related Docs
- Prior ADR (universal contract baseline): `docs/adr/0009-universal-agent-api.md`
- Prior planning pack (baseline): `docs/project_management/next/universal-agent-api/`
- Spec manifest: `docs/project_management/next/claude-code-live-stream-json/spec_manifest.md`
- Decision Register: `docs/project_management/next/claude-code-live-stream-json/decision_register.md`
- Plan: `docs/project_management/next/claude-code-live-stream-json/plan.md`
- Tasks: `docs/project_management/next/claude-code-live-stream-json/tasks.json`
- Session log: `docs/project_management/next/claude-code-live-stream-json/session_log.md`
- Specs (pinned by `spec_manifest.md`):
  - `docs/project_management/next/claude-code-live-stream-json/contract.md`
  - `docs/project_management/next/claude-code-live-stream-json/stream-json-print-protocol-spec.md`
  - `docs/project_management/next/claude-code-live-stream-json/platform-parity-spec.md`
  - `docs/project_management/next/claude-code-live-stream-json/ci_checkpoint_plan.md`
  - `docs/project_management/next/claude-code-live-stream-json/C0-spec.md`
  - `docs/project_management/next/claude-code-live-stream-json/C1-spec.md`
- Impact Map: `docs/project_management/next/claude-code-live-stream-json/impact_map.md`
- Manual Playbook: `docs/project_management/next/claude-code-live-stream-json/manual_testing_playbook.md`

## Executive Summary (Operator)

ADR_BODY_SHA256: f77831dba37ac8c5bcfc5ffb6c10916984e3291aaf0ac8978a0ddf8525050cc4

### Changes (operator-facing)
- Enable live event streaming for Claude Code in the Universal Agent API
  - Existing: `agent_api`’s Claude Code backend emits events only after the `claude` process exits because it buffers stdout before parsing.
  - New: `crates/claude_code` exposes a streaming `--print --output-format stream-json` API that yields parsed JSONL events as they arrive; `agent_api`’s Claude backend forwards these events live and advertises `agent_api.events.live`.
  - Why: Unlocks real-time UX/progress for Claude runs, aligns Claude backend behavior with operator expectations, and preserves Universal Agent API DR-0012 completion safety guarantees (completion resolves only after the event stream is final).
  - Links:
    - Current buffered parsing (to be replaced in `agent_api`): `crates/agent_api/src/backends/claude_code.rs`
    - Current buffered stdout collection in Claude wrapper: `crates/claude_code/src/process.rs`
    - Claude wrapper print entrypoint (will gain streaming alternative): `crates/claude_code/src/client/mod.rs`
    - Run protocol semantics reference (completion vs stream finality): `docs/project_management/next/universal-agent-api/run-protocol-spec.md`

## Problem / Context

- The Universal Agent API (`crates/agent_api`) intentionally provides a unified run contract: an event stream plus a completion future.
- Today, the Claude Code backend in `agent_api` calls `claude_code::ClaudeClient::print(...)`, which buffers all stdout before parsing stream-json lines. This prevents callers from receiving events until the process exits, even if the upstream `claude` CLI is producing JSONL incrementally.
- Downstream orchestrators that rely on event streaming for progress, cancellation UX, or partial outputs cannot treat Claude as a “live” backend under `agent_api`.

## Goals

- Add a first-class streaming API in `crates/claude_code` for `--print --output-format stream-json` that yields parsed stream-json events as they arrive.
- Wire `agent_api`’s Claude backend to use that streaming API and advertise `agent_api.events.live`.
- Preserve Universal Agent API DR-0012 semantics in `agent_api`: `completion` MUST NOT resolve until the event stream is final (or explicitly dropped by the consumer).
- Keep tests fixture/synthetic; no requirement for a real `claude` binary on CI runners.
- Preserve safety posture:
  - `agent_api` MUST NOT emit raw backend lines in events (`AgentWrapperEvent.data` remains bounded and safe-by-default).
  - Parse errors MUST remain redacted (no embedding full raw lines in error messages).

## Non-Goals

- Wrapping Claude’s interactive default/TUI mode (non-`--print` flows).
- Changing the CLI interface or flags of the upstream `claude` binary.
- Forcing tool payload schema parity across backends (Codex vs Claude).
- Introducing a new universal “raw log streaming” surface in `agent_api` (v1 policy forbids raw backend line capture).

## User Contract (Authoritative)

### CLI
- No new CLI commands/flags are introduced by this ADR.
- This ADR changes library behavior only (Rust APIs in `crates/claude_code` and `crates/agent_api`).
- Exit codes: not applicable (library surface).

### Rust API surface (`crates/claude_code`) (normative)

Add a streaming API that yields typed stream-json events as they arrive.

Contract (v1 for this feature):
- The crate MUST expose a “print stream-json streaming handle” type with:
  - an event stream of `Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>` values (in-order)
  - a completion future yielding process completion as `ExitStatus` (no stdout/stderr buffering)
- The streaming API MUST:
  - spawn the `claude` process with `--print --output-format stream-json`
  - read stdout incrementally and parse JSONL lines with `ClaudeStreamJsonParser`
  - yield events in the order parsed from stdout
  - respect configured timeout (same semantics as existing `run_command` timeout)
  - support cancellation by dropping the handle (best-effort process termination)

Stable shape (exact names to be pinned in `contract.md` during planning):
```rust
use std::{future::Future, pin::Pin};

use futures_core::Stream;

use claude_code::{ClaudeCodeError, ClaudePrintRequest, ClaudeStreamJsonEvent, ClaudeStreamJsonParseError};

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

### Rust API behavior (`crates/agent_api`) (normative)

Update the built-in Claude backend (`feature = "claude_code"`) so that:
- It uses the new `claude_code` streaming API to emit `AgentWrapperEvent`s as events arrive.
- It advertises `agent_api.events.live` in `AgentWrapperCapabilities.ids`.
- It continues to enforce Universal Agent API DR-0012 completion gating semantics via the shared run-handle gate.

### Config
- No new config files are introduced.
- Existing backend config precedence remains:
  - backend config provides defaults
  - per-run request overrides defaults
  - request env overrides backend env keys

### Platform guarantees
- Linux/macOS/Windows MUST be supported on GitHub-hosted runners.
- The streaming implementation MUST NOT require PTY allocation (stdout pipe + line parsing only).
- Line ending handling MUST be tolerant of CRLF (parser strips trailing `\r` before JSON parse).

## Architecture Shape

### Components
- `crates/claude_code`:
  - Add a streaming “print stream-json” API on `ClaudeClient`.
  - Introduce a small streaming adapter that:
    - reads stdout as bytes
    - splits into lines
    - parses with `ClaudeStreamJsonParser`
    - yields `Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>` values
- `crates/agent_api`:
  - Update `backends::claude_code` backend to:
    - call `ClaudeClient::print_stream_json(...)`
    - map streamed Claude events to `AgentWrapperEvent` immediately
    - advertise `agent_api.events.live`
    - preserve Universal Agent API DR-0012 via `run_handle_gate::build_gated_run_handle`

### End-to-end flow
- Inputs:
  - `AgentWrapperRunRequest` (prompt, working_dir, timeout, env, extensions)
- Derived state:
  - Claude client builder config (binary path, timeout, env, working_dir)
- Actions:
  - spawn `claude --print --output-format stream-json ...`
  - incrementally parse stdout JSONL → typed Claude events
  - map typed events → `AgentWrapperEvent` and forward on the universal event stream
  - wait for process exit and resolve completion only after the universal stream is final (Universal Agent API DR-0012)
- Outputs:
  - `AgentWrapperRunHandle` with live `events` + gated `completion`

## Sequencing / Dependencies

- Sequencing entry: `docs/project_management/next/sequencing.json` → add new feature track `claude-code-live-stream-json`.
- Prerequisites:
  - Requires the baseline universal API contract (ADR 0009) and existing `agent_api` Claude backend to exist.
- Triad model:
  - Code triad: implement `crates/claude_code` streaming API + wire to `agent_api`.
  - Test triad: synthetic/fixture tests for the streaming adapter + `agent_api` capability advertisement.
  - Integration triad: reconcile to contract/specs and run full workspace gates.

## Security / Safety Posture

- Fail-closed rules:
  - If the `claude` process cannot be spawned, the run MUST fail with a structured error (no panics).
  - If stream-json parsing fails for a line, the system MUST emit a redacted error event (no raw line embedding), and the run MUST continue parsing subsequent lines where feasible.
- `agent_api` invariants:
  - MUST NOT emit raw backend line content into `AgentWrapperEvent.data` (v1 policy).
  - All emitted `AgentWrapperEvent` values MUST obey `event-envelope-schema-spec.md` bounds enforcement.
- Observability:
  - All universal events MUST carry `agent_kind = "claude_code"`.

## Validation Plan (Authoritative)

### Tests
- `crates/claude_code` unit tests:
  - verify streaming adapter yields events incrementally from a synthetic async reader (no external binaries)
  - verify CRLF tolerance and blank-line skipping match `ClaudeStreamJsonParser` behavior
- `crates/agent_api` unit/integration tests:
  - verify `ClaudeCodeBackend.capabilities()` includes `agent_api.events.live`
  - verify mapping invariants remain consistent (TextOutput uses `text`, Error uses `message`, etc.)
- Workspace gates:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-targets --all-features`

### Manual validation
- Manual playbook (required): `docs/project_management/next/claude-code-live-stream-json/manual_testing_playbook.md`
  - Run a real `claude` print stream-json session and confirm:
    - at least one event arrives before process exit
    - `completion` resolves only after events stream finality (Universal Agent API DR-0012)

### Smoke scripts
- A dedicated feature-local smoke set MUST exist under:
  - `docs/project_management/next/claude-code-live-stream-json/smoke/linux-smoke.sh`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/macos-smoke.sh`
  - `docs/project_management/next/claude-code-live-stream-json/smoke/windows-smoke.ps1`

## Rollout / Backwards Compatibility

- Policy: additive changes only (greenfield breaking is allowed, but not required).
- `crates/claude_code`:
  - existing `ClaudeClient::print(...)` remains supported and unchanged
  - streaming API is additive
- `crates/agent_api`:
  - behavior change: Claude backend now advertises live events and emits events earlier
  - contract remains the same shape (run handle with events + completion)

## Decision Summary

- This body of work contains multiple architectural decisions; a Decision Register is required.
- Decision Register entries:
  - `docs/project_management/next/claude-code-live-stream-json/decision_register.md`:
    - DR-0001, DR-0002, DR-0003, DR-0004, DR-0005, DR-0006, DR-0007, DR-0008, DR-0009, DR-0010, DR-0011, DR-0012, DR-0013
