# Workstream D: JSON Streaming and Logging

Objective: Provide a typed, real-time JSONL streaming API for `codex exec --json`, plus optional log teeing, and capture apply/diff outputs.

Scope
- Deserialize ThreadEvent/item lifecycle into Rust types (agent_message, reasoning, command_execution, file_change, mcp_tool_call, web_search, todo_list, errors).
- Stream with backpressure-safe flushing; surface idle timeout errors.
- Support `--output-last-message` and `--output-schema` handling.
- Add opt-in log teeing to files while preserving stdout/stderr controls; honor `RUST_LOG`.
- Return apply/diff stdout/stderr/exit codes.

Constraints
- No behavioral regressions to current client send_prompt; new APIs should be additive.
- Avoid blocking tokio; use async streams/channels.
- Keep parsing deterministic (no ANSI when parsing JSON).

Key references
- Upstream event model from DeepWiki: thread.started, turn.started, item.* (agent_message, reasoning, command_execution, file_change, mcp_tool_call, web_search, todo_list, errors), turn.completed/failed, ThreadEvent::Error.
- Current code: `crates/codex/src/lib.rs` tee_stream, json flag handling.

Deliverables
- Public stream API for `--json` output with typed events.
- Tests for ordering, tool calls, errors, timeouts.
- Example demonstrating real-time consumption and last-message file handling (coordinate with H).
