# C1 Spec — Codex Backend Adapter (feature-gated)

Status: Draft  
Date (UTC): 2026-02-16  
Owner: Universal Agent API triad (C1)

## Scope (required)

Implement a Codex backend for the universal API behind a feature flag.

### In-scope deliverables

- `agent_api` Cargo feature: `codex`
  - When enabled, compiles a Codex backend that depends on `crates/codex`.
- Backend identity:
  - The backend MUST register under `AgentKind` id `codex`.
- Event mapping:
  - Map Codex `ThreadEvent`/item events into `AgentEvent` per `event-envelope-schema-spec.md`.
  - Preserve safety defaults: do not retain raw output by default.

### Event kind mapping (normative)

The Codex backend MUST map events to `AgentEventKind` using the following rules (best-effort):

- `ThreadStarted`, `TurnStarted`, `TurnCompleted`, `TurnFailed` → `Status`
- `Error`, `ItemFailed` → `Error`
- Item payloads / deltas:
  - `AgentMessage`, `Reasoning` → `TextOutput`
  - `CommandExecution`, `FileChange`, `McpToolCall`, `WebSearch` → `ToolCall`
  - `TodoList` → `Status`
  - `Error` → `Error`
- Capability mapping:
  - Expose at least the core `agent_api.run` capability.
  - Expose streaming capability for Codex as `agent_api.events` + backend-specific capability ids as needed.

### Out of scope (explicit)

- Changing `crates/codex` public API.
- Guaranteeing that Codex tool payload schemas match other agents.
- Replacing Codex’s own JSONL parsing contracts (ADR 0005 remains authoritative for Codex-specific parsing).

## Acceptance Criteria (observable)

- With `--features codex` enabled (on `agent_api`):
  - `cargo test -p agent_api` passes (tests are fixture/sample-based).
  - `cargo test --workspace --all-targets --all-features` remains green on Linux.
- `agent_api` without the `codex` feature continues to compile (no unconditional dep).
