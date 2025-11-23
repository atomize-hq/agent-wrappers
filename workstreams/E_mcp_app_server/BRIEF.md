# Workstream E: MCP Server and App Server Support

Objective: Add helpers to launch and interact with `codex mcp-server` and `codex app-server` in headless mode, including codex/codex-reply tool flows and approval/cancel plumbing.

Scope
- Launch helpers: spawn/kill codex mcp-server and app-server over stdio JSON-RPC using bundled binary + app CODEX_HOME.
- MCP tools: wrap `codex` (start session) and `codex-reply` (continue by conversationId) with parameters (prompt, cwd, model, sandbox, approval policy, config map).
- Event handling: stream `codex/event` notifications, approvals (exec/apply), cancellations, task_complete, errors.
- App-server: support `thread/start`, `thread/resume`, `turn/start`, `turn/interrupt`; capture notifications.
- Config: support `[mcp_servers]` add/remove/login/logout (ties to Workstream B/C) but focus here on runtime interaction.

Constraints
- Non-blocking async; clean shutdown on drop.
- Respect env isolation (Workstream A) for CODEX_HOME and binary.

Key references
- DeepWiki: mcp-server uses JSON-RPC over stdio; tools codex/codex-reply; app-server similar with thread/turn methods and task_complete notifications.
- Notifications: approval requests, task_complete, cancel handling.

Deliverables
- Public API to start/stop MCP/app-server and perform tool calls/threads.
- Stream of notifications/events to caller.
- Tests (where feasible via fixture/mocked stdio) and examples (coordinate with H).
