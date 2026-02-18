# C1 Spec — `agent_api` Claude backend live streaming (`agent_api.events.live`)

Status: Draft  
Date (UTC): 2026-02-18  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

## Scope

Wire `crates/agent_api` Claude backend to use the new `crates/claude_code` streaming print API so
universal events are emitted live (before process exit), and advertise `agent_api.events.live`.

In-scope:
- Update the Claude backend (`crates/agent_api/src/backends/claude_code.rs`) to:
  - call `ClaudeClient::print_stream_json(...)`
  - map typed Claude events to `AgentWrapperEvent` as they arrive
  - advertise `agent_api.events.live` in `AgentWrapperCapabilities.ids`
- Preserve DR-0012 completion gating: completion must not resolve until the event stream is final (or dropped).
- Enforce safety posture:
  - do not emit raw backend lines into `AgentWrapperEvent.data`
  - keep parse errors redacted and bounded

## Acceptance Criteria

- Claude backend capabilities include `agent_api.events.live`.
- A fixture/synthetic test proves events can be observed before process exit (no real `claude` binary required).
- Completion semantics remain DR-0012 compliant (completion waits for stream finality or drop).
- Workspace gates pass at integration: `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, relevant tests, `make preflight`.

## Out of Scope

- Any schema changes to the universal event envelope or capability schema beyond advertising `agent_api.events.live`.
- Any requirement to support PTYs or interactive mode.

