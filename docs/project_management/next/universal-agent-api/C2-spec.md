# C2 Spec — Claude Code Backend Adapter (feature-gated)

Status: Draft  
Date (UTC): 2026-02-16  
Owner: Universal Agent API triad (C2)

## Scope (required)

Implement a Claude Code backend for the universal API behind a feature flag.

### In-scope deliverables

- `agent_api` Cargo feature: `claude_code`
  - When enabled, compiles a Claude backend that depends on `crates/claude_code`.
- Backend identity:
  - The backend MUST register under `AgentKind` id `claude_code`.
- Run semantics:
  - Buffered event production is allowed and must be reflected in capabilities (DR-0001).
  - If live streaming is not available, the backend must still return a coherent `AgentEvent` sequence after completion.
- Event mapping:
  - Map Claude stream-json output into `AgentEvent` per `event-envelope-schema-spec.md`.

### Event kind mapping (normative)

The Claude Code backend MUST map stream-json lines into `AgentEventKind` using the following rules (best-effort):

- System/init/other events → `Status`
- Result error events → `Error`
- Assistant content blocks:
  - `tool_use` → `ToolCall`
  - `tool_result` → `ToolResult`
  - text deltas / messages → `TextOutput`
- Unknown/unclassified → `Unknown`

### Out of scope (explicit)

- Wrapping Claude interactive default mode (non-`--print` flows).
- Requiring a real `claude` binary in tests (fixtures/samples only).

## Acceptance Criteria (observable)

- With `--features claude_code` enabled (on `agent_api`):
  - `cargo test -p agent_api` passes (tests are fixture/sample-based).
  - `cargo test --workspace --all-targets --all-features` remains green on Linux.
- `agent_api` without the `claude_code` feature continues to compile (no unconditional dep).
